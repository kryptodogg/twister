import re

with open('src/ml/pointnet_encoder.rs', 'r') as f:
    text = f.read()

# Update dimensions 361 -> 941
text = text.replace('361', '941')

with open('src/ml/pointnet_encoder.rs', 'w') as f:
    f.write(text)
