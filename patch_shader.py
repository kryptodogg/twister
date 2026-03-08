with open("src/visualization/shaders/gaussian_splatting.wgsl", "r") as f:
    content = f.read()

# I see the prompt gave an example of `point_intensity = audio_mag * forensic_weights`.
# Wait, this shader is for radix sorting... Wait, is there another shader?
# "src/visualization/shaders/gaussian_splatting.wgsl"
# Ah, I see the instructions:
# `@uniform forensic: ForensicWeights`
# `@uniform args: DispatchArgs`
