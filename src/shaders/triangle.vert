#version 450 core
#extension GL_ARB_separate_shader_objects : enable

layout (location = 0) in vec3 inPosition;
layout (location = 1) in vec2 inTexCoord;

layout (location = 0) out vec2 fragTexCoord;

layout (push_constant) uniform PushConsts {
    mat4 proj_view;
} pushConsts;

void main() {
    gl_Position = pushConsts.proj_view * vec4(inPosition, 1.0);
    fragTexCoord = inTexCoord;
}
