use crate::client::{
    WekanClient,
    exports::{ExportError, ExportFormat, ExportMetadata, ExportOptions},
};
use std::fs::File;
impl WekanClient {
    pub async fn export_card(
        &self,
        board: &str,
        list: &str,
        card: &str,
        options: ExportOptions<'_>,
        file: &mut File,
    ) -> Result<ExportMetadata, ExportError> {
        if !matches!(options.format, ExportFormat::Xlsx | ExportFormat::Pdf) {
            return Err(super::super::exports::protocol("card exports require xlsx or pdf").into());
        }
        let route = options.format.suffix()[0];
        self.export_resource(
            &[board, "lists", list, "cards", card, route],
            options,
            file,
            false,
        )
        .await
    }
}
