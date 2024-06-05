in vec2 in_vert;
out vec2 uv;

void main() {
    gl_Position = vec4(in_vert * 2 - vec2(1.0, 1.0), 0, 1);
    uv = vec2(in_vert.xy);
}
