// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

uniform sampler2D taccum;
uniform sampler2D trevealage;
uniform sampler2D tcolor;

out vec4 fragColor;

void main() {
    ivec2 C = ivec2(gl_FragCoord.xy);
    vec4 accum = texelFetch(taccum, C, 0);
    float aa = texelFetch(trevealage, C, 0).r;
    vec4 col = texelFetch(tcolor, C, 0);

    float r = accum.a;
    accum.a = aa;
    if (r >= 1.0) {
        fragColor = vec4(col.rgb, 1.0);
    } else {
        vec3 alp = clamp(accum.rgb / clamp(accum.a, 1e-4, 5e4), 0.0, 1.0);
        fragColor = vec4(col.rgb * r  + alp * (1.0 - r), 1.0);
    }
}
