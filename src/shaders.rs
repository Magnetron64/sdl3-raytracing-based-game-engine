use glsl_compiler::glsl;

pub fn fs() -> &'static [u8] {
    let fs: &[u8] = glsl! {type = Fragment, code = {
    #version 440

    layout (location = 0) in vec4 vUV;
    layout (location = 0) out vec4 o_frag_color;

    layout (set = 3, binding = 0) uniform PushConstants
    {
    float aspect;
    vec4 smoothing;
    vec4 cam_pos;
    mat4 view;
    };


    layout(set = 2, binding = 0) buffer Quants{
        uint size;
        vec4 quants[];
    };



    float distance_from_sphere(in vec3 p, in vec3 c, float r)
    {
        return dot((p - c), (p - c)) - (r * r);
        // return length(p - c) - r;

    }

    vec4 map_the_world(in vec3 p)
    {
        float sum = 0;
        vec3 csum = vec3(0.0);

        for (int i = 0; i < size; i += 2) {
            vec3 v = quants[i].xyz;
            float r = quants[i][3];
            vec3 c = quants[i+1].xyz;
            sum += exp2(-(distance_from_sphere(p, v, r) * smoothing[1]));
            csum += c * exp2(-(distance_from_sphere(p, v, r) * smoothing[3]));

        }

        return vec4(-log(max(sum, 0.001)) * smoothing[0], log(clamp(csum, 0.0, 1.0)));
    }

    vec3 calculate_normal(in vec3 p, in float dtc)
    {
        const vec3 small_step = vec3(0.001, 0.0, 0.0);

        float gradient_x = map_the_world(p + small_step.xyy)[0] - dtc;
        float gradient_y = map_the_world(p + small_step.yxy)[0] - dtc;
        float gradient_z = map_the_world(p + small_step.yyx)[0] - dtc;

        vec3 normal = vec3(gradient_x, gradient_y, gradient_z);
        return normalize(normal);
    }

    vec3 diffuse(in vec3 lp, in vec3 cp, float dtc){
        vec3 normal = calculate_normal(cp, dtc);
        vec3 direction_to_light = normalize(cp - lp);

        float diffuse_intensity = max(0.0, dot(normal, direction_to_light));

        return vec3(0.4, 0.4, 0.4) * diffuse_intensity;
    }

    vec3 ray_march(in vec3 ro, in vec3 rd)
    {
        float total_distance_traveled = 0.0;
        const int NUMBER_OF_STEPS = 16;
        const float MINIMUM_HIT_DISTANCE = 0.1;
        const float MAXIMUM_TRACE_DISTANCE = 1000.0;

        for (int i = 0; i < NUMBER_OF_STEPS; ++i)
        {
            vec3 current_position = ro + total_distance_traveled * rd;

            float distance_to_closest = map_the_world(current_position)[0];
            vec3 color = map_the_world(current_position).yzw;

            if (distance_to_closest < MINIMUM_HIT_DISTANCE)
            {
                vec3 lit_sdf = diffuse(vec3(2.0, -5.0, 3.0), current_position, distance_to_closest);
                return lit_sdf+ color;//vec3(0.0, 1.0, 0.0);
            }

            if (total_distance_traveled > MAXIMUM_TRACE_DISTANCE)
            {
                break;
            }
            total_distance_traveled += distance_to_closest;
        }
        return vec3(0.0, 0.5, 0.5);
    }

    void main() {
        vec2 uv = vUV.xy;
        vec3 uvp = (view * vec4(uv.x * aspect, uv.y, -aspect, 0.0)).xyz;
        vec3 ro = cam_pos.xyz;
        vec3 rd = normalize(uvp);

        vec3 shaded_color = ray_march(ro, rd);

        o_frag_color = vec4(shaded_color.x, shaded_color.y, shaded_color.z, 1.0);
    }
        }};

    fs
}

pub fn vert() -> &'static [u8] {
    let vert: &[u8] = glsl! {type = Vertex, code = {
            #version 440
    layout (location = 0) out vec4 vUV;

    void main(void) {
        if(gl_VertexIndex == 0) {
            gl_Position = vec4(-1.0, -1.0, 0.0, 1.0);
            vUV = gl_Position;
        } else if(gl_VertexIndex == 1) {
            gl_Position = vec4(1.0f, -1.0f, 0.f, 1.f);
            vUV = gl_Position;
        } else if(gl_VertexIndex == 2) {
            gl_Position = vec4(1.0, 1.0, 0.0, 1.0);
            vUV = gl_Position;
        }else if(gl_VertexIndex == 3) {
            gl_Position = vec4(1.0, 1.0, 0.0, 1.0);
            vUV = gl_Position;
        } else if(gl_VertexIndex == 4) {
            gl_Position = vec4(-1.0, 1.0, 0.0, 1.0);
            vUV = gl_Position;
        } else if(gl_VertexIndex == 5) {
            gl_Position = vec4(-1.0f, -1.0, 0.0, 1.0);
            vUV = gl_Position;
        }

    }

        }};

    vert
}

pub fn win_fs() -> &'static [u8] {
    let fs: &[u8] = glsl! {type = Fragment, code = {

    #version 440
    layout(location = 0) in vec2 v_texCoord;
    layout(location = 0) out vec4 fragColor;

    layout(set = 2, binding = 0) uniform sampler2D u_texture;

    void main() {
       fragColor = texture(u_texture, v_texCoord);
    }

    }};
    fs
}

pub fn win_vert() -> &'static [u8] {
    let vert: &[u8] = glsl! {type = Vertex, code = {
                #version 440
    layout(location = 0) out vec2 v_texCoord;

    vec2 positions[6] = vec2[](
        vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 1.0),
        vec2(1.0, -1.0),  vec2(1.0, 1.0),  vec2(-1.0, 1.0)
    );

    vec2 texCoords[6] = vec2[](
        vec2(0.0, 1.0), vec2(1.0, 1.0), vec2(0.0, 0.0),
        vec2(1.0, 1.0), vec2(1.0, 0.0), vec2(0.0, 0.0)
    );

    void main() {
        gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
        v_texCoord = texCoords[gl_VertexIndex];
    }
            }};

    vert
}
