// Fixture only: use the same DDP collection inserts as the v11.06 checklist UI.
// Run in the pinned Wekan container; receive the disposable test token on stdin.
const fs = require('node:fs');
const input = JSON.parse(fs.readFileSync(0, 'utf8'));
const socket = new WebSocket('ws://127.0.0.1:8080/websocket');
const pending = new Map();
let sequence = 0;

const connected = new Promise((resolve, reject) => {
  socket.addEventListener('open', () => {
    socket.send(JSON.stringify({ msg: 'connect', version: '1', support: ['1'] }));
  });
  socket.addEventListener('error', () => reject(new Error('DDP fixture connection failed')));
  socket.addEventListener('message', event => {
    const message = JSON.parse(event.data);
    if (message.msg === 'connected') resolve();
    if (message.msg === 'ping') {
      socket.send(JSON.stringify({ msg: 'pong', ...(message.id ? { id: message.id } : {}) }));
    }
    if (message.msg === 'result' && pending.has(message.id)) {
      const task = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) task.reject(new Error(JSON.stringify(message.error)));
      else task.resolve(message.result);
    }
  });
});

function method(name, params) {
  return new Promise((resolve, reject) => {
    const id = String(++sequence);
    pending.set(id, { resolve, reject });
    socket.send(JSON.stringify({ msg: 'method', method: name, params, id }));
  });
}

const timeout = setTimeout(() => {
  console.error('DDP checklist fixture timed out');
  socket.close();
  process.exit(1);
}, 30000);

(async () => {
  try {
    await connected;
    const login = await method('login', [{ resume: input.token }]);
    await method('/checklists/insert', [{
      _id: input.checklist_id, cardId: input.card_id, title: 'UI checklist', sort: 0,
    }]);
    await method('/checklistItems/insert', [{
      _id: input.item_id, cardId: input.card_id, checklistId: input.checklist_id,
      title: 'UI item', sort: 0,
    }]);
    console.log(JSON.stringify({
      checklist_id: input.checklist_id, item_id: input.item_id, user_id: login.id,
    }));
  } finally {
    clearTimeout(timeout);
    socket.close();
  }
})().catch(error => {
  console.error(error.message);
  process.exitCode = 1;
});
