// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

uniform sampler2DArray textures;

in vec3 vColor;
in vec4 vTextureInfo;
in vec2 vTextureOffset;
in float vAtlas;
in vec3 vLighting;

#ifdef ES
layout(location = 2) out vec4 fragColor;
#else
out vec4 fragColor;
#endif

#ifdef ES
layout(location = 0) out vec4 accum;
layout(location = 1) out float revealage;
#else
out vec4 accum;
out float revealage;
#endif

#include lookup_texture

void main() {
    vec4 col = atlasTexture();
    #ifndef alpha
    if (col.a < 0.5) discard;
    #endif
    col *= vec4(vColor, 1.0);
    col.rgb *= vLighting;

    #ifndef alpha
    fragColor = col;
    #else
    float z = gl_FragCoord.z;
    float al = col.a;
    float weight = pow(al + 0.01f, 4.0f) +
                     max(0.01f, min(3000.0f, 0.3f / (0.00001f + pow(abs(z) / 800.0f, 4.0f))));
    accum = vec4(col.rgb * al * weight, al);
    revealage = weight * al;
    #endif
}
