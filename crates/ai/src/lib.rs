//! Integração com provedores de IA.
//!
//! O ponto de entrada principal é [`Client`], que roteia chamadas para o
//! provider correto com base no prefixo do nome do modelo (por exemplo,
//! `"ollama/qwen3:8b"` ou `"whisper/whisper-1"`).
//!
//! ```ignore
//! let client = ai::Client::builder()
//!     .ollama("http://localhost:11434")
//!     .whisper("http://localhost:9000/v1/audio/transcriptions")
//!     .build();
//!
//! client.chat("ollama/qwen3:8b", &messages, &tools).await?;
//! client.embed_many("ollama/qwen3-embedding:4b", &texts, None).await?;
//! client.transcribe("whisper/whisper-1", bytes, "audio.ogg", "audio/ogg").await?;
//! ```
//!
//! Os módulos [`ollama`] e [`whisper`] também ficam expostos para quem
//! quiser usar os clients de baixo nível diretamente.

mod chat;
mod client;

pub mod gemini;
pub mod ollama;
pub mod whisper;

pub use chat::{
    ChatMessage, ChatRequest, ChatResponse, ChatTool, StructuredRequest, StructuredResponse,
};
pub use client::{Client, ClientBuilder};
