import re

with open('src/state.rs', 'r') as f:
    content = f.read()

# Mock FeatureFlags for state.rs so it compiles without ml
content = content.replace("use crate::ml::modular_features::FeatureFlags;", """
#[derive(Clone)]
pub struct FeatureFlags {
    pub enhanced_audio: bool,
    pub sparse_pdm: bool,
    pub coherence: bool,
    pub mamba_siren: bool,
}
impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enhanced_audio: true,
            sparse_pdm: false,
            coherence: false,
            mamba_siren: false,
        }
    }
}
""")

with open('src/state.rs', 'w') as f:
    f.write(content)
