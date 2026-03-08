//! Loss Functions for Point Mamba Training
//!
//! **Modules**:
//! - chamfer_distance: Bidirectional nearest-neighbor loss for point clouds
//!
//! **Fusion Strategy**:
//! Combined Chamfer-Huber loss handles both geometry reconstruction and semantic robustness:
//! ```
//! L_total = L_CD + λ·L_Huber (λ=0.5)
//! ```

pub mod chamfer_distance;

pub use chamfer_distance::{ChamferDistance, HuberLoss};
