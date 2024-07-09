#version 460

// Fragment inputs.
layout (location = 0) in vec3 fragColor;
layout (location = 1) in vec2 fragTexCoord;

// Fragment outputs.
layout (location = 0) out vec4 outColor;

// Bindings.
layout (binding = 1) uniform sampler2D texSampler;

void main() {
    outColor = texture(texSampler, fragTexCoord) * vec4(fragColor, 1.0);
}
