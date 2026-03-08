import re

with open("src/ml/mamba_block.rs", "r") as f:
    content = f.read()

# Replace `TensorData::random(..., ..., &device)` with `&mut rand::thread_rng()`
# wait, TensorData::random signature is `random<E, R, S>(shape, dist, rng)`
# In the code it does: `TensorData::random([128, 128], burn::tensor::Distribution::Default, &device)` which is wrong for burn v0.14+ where it takes a rng instead of a device.
content = re.sub(r'TensorData::random\(([^,]+), burn::tensor::Distribution::Default, &device\)',
                 r'TensorData::random(\1, burn::tensor::Distribution::Default, &mut rand::thread_rng())',
                 content)

# We also need to make sure `rand` is in scope or we use `rand::thread_rng`
with open("src/ml/mamba_block.rs", "w") as f:
    f.write(content)
