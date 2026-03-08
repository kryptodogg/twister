with open("src/ml/point_mamba.rs", "r") as f:
    content = f.read()

bad = "#[derive(Module, Debug)]"
good = "#[derive(burn::module::Module, Debug)]"

content = content.replace(bad, good)

with open("src/ml/point_mamba.rs", "w") as f:
    f.write(content)
