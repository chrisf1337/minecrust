#version 450 core
out vec4 fragColor;

in VtxOut {
    vec3 pos;
    vec2 texCoord;
} vtxOut;

uniform sampler2D ourTexture;

void main() {
    fragColor = texture(ourTexture, vtxOut.texCoord);
}
