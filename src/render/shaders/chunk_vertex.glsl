// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

in vec3 aPosition;
in vec4 aTextureInfo;
in vec3 aTextureOffset;
in vec3 aColor;
in vec2 aLighting;

uniform mat4 perspectiveMatrix;
uniform mat4 cameraMatrix;
uniform ivec3 offset;
uniform float lightLevel;
uniform float skyOffset;

out vec3 vColor;
out vec4 vTextureInfo;
out vec2 vTextureOffset;
out float vAtlas;
out vec3 vLighting;

#include get_light

void main() {
    vec3 pos = vec3(aPosition.x, -aPosition.y, aPosition.z);
    vec3 o = vec3(float(offset.x), -float(offset.y) / 4096.0, float(offset.z));
    gl_Position = perspectiveMatrix * cameraMatrix * vec4(pos + o * 16.0, 1.0);

    vColor = aColor;
    vTextureInfo = aTextureInfo;
    vTextureOffset = aTextureOffset.xy / 16.0;
    vAtlas = aTextureOffset.z;

    vLighting = getLight(aLighting / (4000.0));
}
