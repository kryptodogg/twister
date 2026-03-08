import re

with open('src/ml/point_mamba.rs', 'r') as f:
    text = f.read()

text = text.replace('use burn::prelude::Module;', 'use burn::module::Module;')
text = text.replace('output = block.forward(output);', 'output = block.forward(&output);')

with open('src/ml/point_mamba.rs', 'w') as f:
    f.write(text)

with open('src/ml/mamba_block.rs', 'r') as f:
    text = f.read()

text = text.replace('let delta = gate_logits.clone().sigmoid();', 'let delta = burn::tensor::activation::sigmoid(gate_logits.clone());')
text = text.replace('original.add(&output)', 'original.add(output)')

with open('src/ml/mamba_block.rs', 'w') as f:
    f.write(text)

with open('src/ml/pointnet_encoder.rs', 'r') as f:
    text = f.read()

text = text.replace('use burn::tensor::{Data, Distribution, Tensor, TensorData};', 'use burn::tensor::{Distribution, Tensor};')

with open('src/ml/pointnet_encoder.rs', 'w') as f:
    f.write(text)

with open('src/ml/point_decoder.rs', 'r') as f:
    text = f.read()

text = text.replace('use burn::tensor::{Distribution, Tensor, TensorData};', 'use burn::tensor::{Distribution, Tensor};')

with open('src/ml/point_decoder.rs', 'w') as f:
    f.write(text)
