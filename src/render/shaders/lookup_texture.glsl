// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

const float invAtlasSize = 1.0 / 2048.0;

vec4 atlasTexture() {
    vec2 tPos = vTextureOffset;
    tPos = clamp(tPos, vec2(0.1), vTextureInfo.zw - 0.1);
    tPos += vTextureInfo.xy;
    tPos *= invAtlasSize;
    return texture(textures, vec3(tPos, vAtlas));
}
