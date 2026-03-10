import re

with open('src/forensic.rs', 'r') as f:
    f_content = f.read()

# We need to find the second occurrence of AnomalyGateDecision { and remove it.
# Let's locate the exact lines.

lines = f_content.split('\n')
enum_start = -1
for i, line in enumerate(lines):
    if line.strip() == "pub enum ForensicEvent {":
        enum_start = i
        break

if enum_start != -1:
    # Read through enum, count AnomalyGateDecision {
    in_enum = True
    braces = 0
    gate_decision_starts = []

    # Simple regex find for AnomalyGateDecision {
    for i in range(enum_start, len(lines)):
        if "AnomalyGateDecision {" in lines[i]:
            gate_decision_starts.append(i)

    if len(gate_decision_starts) > 1:
        # We need to remove the second block
        start_idx = gate_decision_starts[1]

        # Find the end of this block
        end_idx = start_idx
        braces = 1
        for j in range(start_idx + 1, len(lines)):
            if "{" in lines[j]:
                braces += 1
            if "}" in lines[j]:
                braces -= 1
                if braces == 0:
                    end_idx = j
                    break

        # Remove from start_idx to end_idx (inclusive)
        if "}," in lines[end_idx]:
            # if it's like `},` keep it accurate
            pass

        del lines[start_idx:end_idx+1]

f_content = '\n'.join(lines)
with open('src/forensic.rs', 'w') as f:
    f.write(f_content)
