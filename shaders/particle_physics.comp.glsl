#version 450

layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

struct Particle {
    vec2 position;
    vec2 velocity;
};

layout(set = 0, binding = 0) buffer Data {
    Particle data[];
} vertices;

layout(set = 1, binding = 0) uniform UniformBufferObject {
    vec2 target;
    float delta_time;
} ubo;

void main() {
    float gravity = 1.0;

    uint idx = gl_GlobalInvocationID.x;
    vec2 delta = normalize(ubo.target - vertices.data[idx].position) * gravity;

    vertices.data[idx].velocity += delta;

    vertices.data[idx].position += vertices.data[idx].velocity * ubo.delta_time;
}
