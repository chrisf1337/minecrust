#version 450 core
#extension GL_ARB_separate_shader_objects : enable

layout (location = 0) in uint charIdx;
layout (location = 1) in vec2 inPos;
layout (location = 2) in vec2 inUv;
layout (location = 3) in vec4 inColor;

layout (location = 0) out vec2 outUv;
layout (location = 1) out vec4 outColor;

// layout (push_constant) uniform PushConsts {
//     mat4 view_mat;
//     mat4 proj_mat;
// } pushConsts;

void main() {
    // gl_Position = pushConsts.proj_mat * pushConsts.view_mat * vec4(inPosition, 0.0, 1.0);
    gl_Position = vec4(inPos, 0.0, 1.0);
    outUv = inUv;
    outColor = inColor;
}
