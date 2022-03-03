// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

uniform sampler2DArray textures;
uniform vec4 colorMul[10];

in vec4 vTextureInfo;
in vec2 vTextureOffset;
in float vAtlas;
in float vID;

out vec4 fragColor;

#include lookup_texture

void main() {
	vec4 col = atlasTexture();
	if (col.a <= 0.05) discard;
	fragColor = col * colorMul[int(vID)];
}
