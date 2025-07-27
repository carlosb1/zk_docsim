use std::io::ErrorKind;
use anyhow::Error;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct DocumentEntry {
    pub name: String,
    pub content: String,
}

impl DocumentEntry {
    pub fn new(name: &str, content: &str) -> Self {
        Self { name: name.to_string(), content: content.to_string() }
    }
}

pub struct ModelEmbed{
    model: TextEmbedding,
}

impl ModelEmbed {
    pub fn new() -> Self {
        let model = TextEmbedding::try_new(InitOptions::new(EmbeddingModel::AllMiniLML6V2)).expect("Failed to create Embedding");
        ModelEmbed{model}
    }

    pub fn calculate_one_embed(&self, document_entry: DocumentEntry) -> anyhow::Result<Vec<f32>> {
        let batch = vec![serde_json::to_string(&document_entry)?];
        let binding =  self.model.embed(batch, None)?;
        let embedding =binding.first().expect("It can not calculate the embedding");
        Ok(embedding.clone())
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_calculate_embedding() {
        let model = ModelEmbed::new();
        let results = model.calculate_one_embed(DocumentEntry::new("example", "encoding")).unwrap();
        println!("{:?}", results);
    }
 }