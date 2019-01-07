#version 450 core
#extension GL_ARB_separate_shader_objects : enable

layout (location = 0) in vec3 inPos;
layout (location = 1) in vec2 inUv;

layout (location = 0) out vec2 outUv;

layout (push_constant) uniform PushConsts {
    mat4 proj_view;
} pushConsts;

void main() {
    gl_Position = pushConsts.proj_view * vec4(inPos, 1.0);
    outUv = inUv;
}
