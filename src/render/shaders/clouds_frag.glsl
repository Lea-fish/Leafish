// Copyright 2021-2022 Leafish Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

in vec4 fColor;
in vec3 fLighting;

out vec4 fragColor;

void main() {
	vec4 col = fColor;
	col.rgb *= fLighting;
	fragColor = col;
}
