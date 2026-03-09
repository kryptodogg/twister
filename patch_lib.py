import re

with open('src/lib.rs', 'r') as f:
    content = f.read()

content = content.replace("// pub mod state;", "pub mod state;")

with open('src/lib.rs', 'w') as f:
    f.write(content)
