import re

with open("src/ml/mamba_block.rs", "r") as f:
    content = f.read()

# Just put the random initialization behind a cfg test check if needed, or fix it properly.
# The error is that `&device` was passed but `&mut Rng` was expected in `TensorData::random`.
# wait, if I changed `Tensor::from_data(TensorData::random(...))` to `Tensor::random(...)`, what did I change it to?
# Ah! In the actual code `MambaBlock::new` or tests.
# The errors are on lines 75, 84, 106, 125, 134, 151, 164. Let's see what is there.

content = re.sub(r'TensorData::random\(([^,]+),\s*([^,]+),\s*&device\)',
                 r'Tensor::random(\1, \2, &device).into_data()',
                 content)

with open("src/ml/mamba_block.rs", "w") as f:
    f.write(content)


with open("src/ml/pointnet_encoder.rs", "r") as f:
    content = f.read()

content = re.sub(r'TensorData::random\(([^,]+),\s*([^,]+),\s*&device\)',
                 r'Tensor::<Backend, 2>::random(\1, \2, &device).into_data()',
                 content)

with open("src/ml/pointnet_encoder.rs", "w") as f:
    f.write(content)

with open("src/ml/point_decoder.rs", "r") as f:
    content = f.read()

content = re.sub(r'TensorData::random\(([^,]+),\s*([^,]+),\s*&device\)',
                 r'Tensor::<Backend, 3>::random(\1, \2, &device).into_data()',
                 content)

with open("src/ml/point_decoder.rs", "w") as f:
    f.write(content)
