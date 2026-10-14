use super::{Command, Stdio, Write};
use serde_json::{Value, json};

pub(crate) fn fixture_command(
    service: &str,
    executable: &str,
    script: &str,
    input: &Value,
) -> Value {
    let mut args = vec!["compose", "exec", "-T", service, executable];
    if executable == "mongosh" {
        args.extend(["--quiet", "--eval"]);
    } else {
        args.push("-e");
    }
    let wrapped = format!(
        "(async()=>{{ const chunks=[]; for await(const chunk of process.stdin) chunks.push(Buffer.from(chunk)); const p=JSON.parse(Buffer.concat(chunks).toString('utf8')); {script} }})().catch(error=>{{console.error(error); process.exitCode=1;}})"
    );
    args.push(&wrapped);
    let mut process = Command::new("docker")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input_written = process
        .stdin
        .take()
        .unwrap()
        .write_all(serde_json::to_string(input).unwrap().as_bytes());
    let output = process.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    input_written.unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}

pub(crate) enum StoredFile {
    Filesystem(String),
    GridFs(String),
}

pub(crate) fn seed(input: &Value) -> StoredFile {
    // Test-only persisted metadata and genuine bytes. The pinned REST upload
    // cannot seed fixtures: it returns 408 before attempting insertion.
    if input["storage"] == "fs" {
        fixture_command(
            "wekan",
            "node",
            r#"
          const fs=require('fs');
          fs.mkdirSync('/data/files/attachments',{recursive:true});
          fs.writeFileSync(p.path,Buffer.from(p.data,'base64')); console.log('{}');
        "#,
            input,
        );
    }
    let result = fixture_command(
        "mongodb",
        "mongosh",
        r#"
      const w=db.getSiblingDB('wekan');
      const data=Buffer.from(p.data,'base64'); const now=new Date();
      const version={size:data.length,type:'application/octet-stream',storage:p.storage};
      if(p.storage==='gridfs') {
        const oid=new ObjectId(); version.meta={gridFsFileId:oid.toHexString()};
        w.getCollection('attachments.files').insertOne({_id:oid,length:data.length,chunkSize:255*1024,uploadDate:now,filename:'fixture.bin'});
        for(let n=0,offset=0;offset<data.length;n++,offset+=255*1024) {
          const chunk=data.subarray(offset,offset+255*1024);
          w.getCollection('attachments.chunks').insertOne({_id:new ObjectId(),files_id:oid,n,data:BinData(0,chunk.toString('base64'))});
        }
      } else {version.path=p.path;}
      w.attachments.insertOne({_id:p.id,name:'fixture.bin',size:data.length,type:'application/octet-stream',
        extension:'bin',extensionWithDot:'.bin',isImage:false,userId:p.user_id,uploadedAt:now,
        meta:p.background ? {boardId:p.board_id,source:'cli-test'} : {boardId:p.board_id,swimlaneId:p.swimlane_id,listId:p.list_id,cardId:p.card_id,source:'cli-test'},
        versions:{original:version,thumbnail:{}}});
      if(p.background) w.boards.updateOne({_id:p.board_id},{$set:{backgroundImageId:p.id,backgroundImageURL:'/fixture-background'}});
      print(JSON.stringify({gridfs_file_id:version.meta?.gridFsFileId || null}));
    "#,
        input,
    );
    if input["storage"] == "fs" {
        StoredFile::Filesystem(input["path"].as_str().unwrap().to_owned())
    } else {
        StoredFile::GridFs(result["gridfs_file_id"].as_str().unwrap().to_owned())
    }
}

pub(crate) fn assert_stored_file_deleted(file: &StoredFile) {
    // Poll only in the live test: v11.06 initiates physical cleanup asynchronously.
    // Check before board/stack cleanup can independently remove the fixture bytes.
    match file {
        StoredFile::Filesystem(path) => {
            let result = fixture_command(
                "wekan",
                "node",
                r#"
              const fs=require('fs'); const deadline=Date.now()+10000;
              while(fs.existsSync(p.path) && Date.now()<deadline) {
                await new Promise(resolve=>setTimeout(resolve,100));
              }
              console.log(JSON.stringify({exists:fs.existsSync(p.path)}));
            "#,
                &json!({"path":path}),
            );
            assert_eq!(
                result["exists"], false,
                "attachment file remained at {path} after the 10-second cleanup deadline"
            );
        }
        StoredFile::GridFs(id) => {
            let result = fixture_command(
                "mongodb",
                "mongosh",
                r#"
              const w=db.getSiblingDB('wekan'); const oid=new ObjectId(p.id);
              const remaining=async()=>({
                files:await w.getCollection('attachments.files').countDocuments({_id:oid}),
                chunks:await w.getCollection('attachments.chunks').countDocuments({files_id:oid})
              });
              const deadline=Date.now()+10000; let counts=await remaining();
              while((counts.files || counts.chunks) && Date.now()<deadline) {
                await new Promise(resolve=>setTimeout(resolve,100)); counts=await remaining();
              }
              print(JSON.stringify(counts));
            "#,
                &json!({"id":id}),
            );
            assert_eq!(
                result,
                json!({"files":0,"chunks":0}),
                "GridFS bytes remained for {id} after the 10-second cleanup deadline"
            );
        }
    }
}
