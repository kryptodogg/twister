import re

with open("src/ml/mamba_block.rs", "r") as f:
    content = f.read()

# Undo the previous thread_rng substitution
content = content.replace("burn::tensor::Distribution::Default, &mut rand::thread_rng()", "burn::tensor::Distribution::Default, &device")

# Actually, the error is in tests because of `device`. The tests use `NdArrayBackend`.
# So let's look at `point_mamba.rs` tests.
# The error says "expected mutable reference &mut _ found reference &_".
# Ah! In burn 0.14+ (or whatever version), you pass `&device` but maybe we need `&device` to not be `&mut` or something?
# Wait! In burn 0.21, `TensorData::random(shape, dist, rng)` actually takes `&mut impl Rng`.
# If `device` was passed, that means it's a completely different signature.
# Let's check `Tensor::random(shape, dist, &device)`!
# Yes, `TensorData::random` takes `rng`. `Tensor::random` takes `device`.
# Wait, the tests are using `Tensor::from_data(TensorData::random(..., &device), &device)`.
# Let's replace `Tensor::from_data(TensorData::random(shape, dist, &device), &device)` with `Tensor::random(shape, dist, &device)`!

content = re.sub(r'Tensor::from_data\(\s*TensorData::random\(([^,]+),\s*([^,]+),\s*&device\),\s*&device\s*\)',
                 r'Tensor::random(\1, \2, &device)',
                 content)

with open("src/ml/mamba_block.rs", "w") as f:
    f.write(content)


with open("src/ml/pointnet_encoder.rs", "r") as f:
    content = f.read()

content = re.sub(r'Tensor::from_data\(\s*TensorData::random\(([^,]+),\s*([^,]+),\s*&device\),\s*&device,\s*\)',
                 r'Tensor::random(\1, \2, &device)',
                 content)

with open("src/ml/pointnet_encoder.rs", "w") as f:
    f.write(content)

with open("src/ml/point_decoder.rs", "r") as f:
    content = f.read()

content = re.sub(r'Tensor::from_data\(\s*TensorData::random\(([^,]+),\s*([^,]+),\s*&device\),\s*&device,\s*\)',
                 r'Tensor::random(\1, \2, &device)',
                 content)

with open("src/ml/point_decoder.rs", "w") as f:
    f.write(content)
