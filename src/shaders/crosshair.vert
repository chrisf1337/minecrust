#version 450 core
#extension GL_ARB_separate_shader_objects : enable

layout (location = 0) in vec2 inPos;
layout (location = 1) in vec2 inUv;

layout (location = 0) out vec2 outUv;

layout (binding = 0) uniform Uniforms {
    mat4 screen_space_normalize_mat;
} uniforms;

void main() {
    gl_Position = vec4(inPos, 0.0, 1.0);
    outUv = inUv;
}
