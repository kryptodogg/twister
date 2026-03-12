//! Hardware I/O — V3 DMA and Stream Management
//!
//! # V3 Architecture Notes
//! - device_manager deleted — was using DirtyFlags which was removed
//! - dma_vbuffer is the primary IQ ingestion path

// pub mod device_manager; — deleted, being rewritten
pub mod dma_vbuffer;

// #[cfg(feature = "rtlsdr")] — deleted with device_manager
pub use dma_vbuffer::IqDmaGateway;
