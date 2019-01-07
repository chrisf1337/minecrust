#version 450 core
#extension GL_ARB_separate_shader_objects : enable

layout (location = 0) in vec2 inUv;
layout (location = 1) in vec4 inColor;
layout (location = 2) flat in uint inCharIdx;

layout (location = 0) out vec4 outColor;

layout (binding = 1) uniform sampler texSampler;
layout (binding = 2) uniform texture2D glyphTextures[256];

void main() {
    vec4 color = inColor * texture(sampler2D(glyphTextures[inCharIdx], texSampler), inUv);
    if (color.a <= 0.3) {
        discard;
    }
    outColor = color;
}
