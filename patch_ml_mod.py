with open('src/ml/mod.rs', 'r') as f:
    content = f.read()

content = content.replace("pub mod spectral_frame;\n", "", 1)
content = content.replace("pub mod anomaly_gate;\n", "", 1)

with open('src/ml/mod.rs', 'w') as f:
    f.write(content)
