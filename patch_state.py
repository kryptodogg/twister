import re
with open("src/state.rs", "r") as f:
    text = f.read()

target = r'(pub fn new_test\(\) -> Arc<Self> \{\n        Arc::new\(Self \{)'
replacement = r'\1\n            material_library: std::sync::Arc::new(tokio::sync::Mutex::new(crate::materials::material_library::MaterialLibrary::default())), '

if 'material_library: std::sync::Arc::new(tokio::sync::Mutex::new(crate::materials::material_library::MaterialLibrary::default())),' not in text:
    text = re.sub(target, replacement, text, count=1)

with open("src/state.rs", "w") as f:
    f.write(text)
