#version 450 core
out vec4 fragColor;

in VtxOut {
    vec2 texCoord;
} vtxOut;

uniform sampler2D tex;

void main() {
    fragColor = texture(tex, vtxOut.texCoord);
}
