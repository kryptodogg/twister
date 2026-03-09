import re
<<<<<<< HEAD

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
=======
with open("src/state.rs", "r") as f:
    text = f.read()

target = r'(pub fn new_test\(\) -> Arc<Self> \{\n        Arc::new\(Self \{)'
replacement = r'\1\n            material_library: std::sync::Arc::new(tokio::sync::Mutex::new(crate::materials::material_library::MaterialLibrary::default())), '

if 'material_library: std::sync::Arc::new(tokio::sync::Mutex::new(crate::materials::material_library::MaterialLibrary::default())),' not in text:
    text = re.sub(target, replacement, text, count=1)

with open("src/state.rs", "w") as f:
    f.write(text)
>>>>>>> origin/jules-track-ff-materials-14124701718082983222
