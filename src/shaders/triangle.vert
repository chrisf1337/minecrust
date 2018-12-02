#version 450 core
#extension GL_ARB_separate_shader_objects : enable

layout (location = 0) in vec3 inPosition;
layout (location = 1) in vec2 inTexCoord;

layout (location = 0) out vec2 fragTexCoord;

layout (binding = 0) uniform UniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} ubo;

layout (push_constant) uniform PushConsts {
    mat4 view_mat;
    mat4 proj_mat;
} pushConsts;

void main() {
    gl_Position = pushConsts.proj_mat * pushConsts.view_mat * vec4(inPosition, 1.0);
    fragTexCoord = inTexCoord;
}
