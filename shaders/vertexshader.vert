#version 430 core

layout(location = 0) in vec2 aPos;
layout(location = 1) in vec2 aOffset;

out vec2 relativePos;
flat out uint vInstanceID;

uniform vec2 u_resolution;
uniform vec4 u_maxdim;

void main() {
    vec2 normalizedPos = vec2(
        2.0 * aPos.x / u_resolution.x,
        -2.0 * aPos.y / u_resolution.y
    );
    
    vec2 worldPos = normalizedPos + aOffset;
    
    gl_Position = vec4(worldPos, 0.0, 1.0);
    relativePos = vec2(u_maxdim.z + aPos.x, u_maxdim.y - aPos.y + u_maxdim.w);
    
    vInstanceID = uint(gl_InstanceID);
}
