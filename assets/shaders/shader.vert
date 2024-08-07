#version 460

// Vertex inputs.
layout(location = 0) in vec2 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 2) in vec2 inTexCoord;

// Vertex outputs.
layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec2 fragTexCoord;

// Bindings.
layout(binding = 0) uniform UniformData {
    mat4 model;
    mat4 view;
    mat4 proj;
} ubo;

void main() {
    gl_Position = ubo.proj * ubo.view * ubo.model * vec4(inPosition, 0.0, 1.0);
    fragColor = inColor;
    fragTexCoord = inTexCoord;
}