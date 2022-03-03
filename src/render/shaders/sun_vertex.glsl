// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

in vec3 aPosition;
in vec4 aTextureInfo;
in ivec3 aTextureOffset;
in vec4 aColor;
in int id;

uniform mat4 perspectiveMatrix;
uniform mat4 cameraMatrix;
uniform mat4 modelMatrix[10];
uniform float lightLevel;
uniform float skyOffset;
uniform vec2 lighting;

out vec4 vTextureInfo;
out vec2 vTextureOffset;
out float vAtlas;
out float vID;

void main() {
	vec3 pos = vec3(aPosition.x, -aPosition.y, aPosition.z);
	gl_Position = perspectiveMatrix * cameraMatrix * modelMatrix[id] * vec4(pos, 1.0);

	vTextureInfo = aTextureInfo;
	vTextureOffset = vec2(aTextureOffset.xy) / 16.0;
	vAtlas = float(aTextureOffset.z);
	vID = float(id);
}
