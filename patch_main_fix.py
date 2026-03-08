with open("src/main.rs", "r") as f:
    lines = f.readlines()

out_lines = []
skip = False
for i, line in enumerate(lines):
    if "let (feature_tx, feature_rx)" in line:
        if skip:
            continue
        skip = True
    out_lines.append(line)

with open("src/main.rs", "w") as f:
    f.writelines(out_lines)
