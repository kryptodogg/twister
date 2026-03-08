import re

with open('src/ml/mamba_block.rs', 'r') as f:
    text = f.read()

text = text.replace('self.output_c.clone().unsqueeze_dim(0).unsqueeze_dim(0)', 'self.output_c.clone().unsqueeze_dim::<2>(0).unsqueeze_dim::<3>(0)')

with open('src/ml/mamba_block.rs', 'w') as f:
    f.write(text)

with open('src/ml/point_mamba.rs', 'r') as f:
    text = f.read()

text = text.replace('#[derive(Module, Debug)]', '#[derive(burn::module::Module, Debug)]')

with open('src/ml/point_mamba.rs', 'w') as f:
    f.write(text)
