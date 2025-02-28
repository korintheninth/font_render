#version 430 core
out vec4 FragColor;

flat in uint vInstanceID;
in vec2 relativePos;

uniform vec2 u_resolution;
uniform float u_scale;

struct Point {
    int x;
    int y;
    int flag;
};

struct Glyph {
    int xmin;
    int ymin;
    int xmax;
    int ymax;
    int num_points;
    int num_contours;
    int points_offset;
    int contours_offset;
};

layout(std430, binding = 0) buffer GlyphBuffer {
    Glyph glyphs[];
};

layout(std430, binding = 1) buffer PointBuffer {
    Point points[];
};

layout(std430, binding = 2) buffer ContourBuffer {
    uint contours[];
};


bool is_inbound(float x, float y, float xmin, float ymin, float xmax, float ymax) {
    return x >= xmin && x <= xmax && y >= ymin && y <= ymax;
}

bool bit_set(int value, int bit) {
    return (value & (1 << bit)) != 0;
}

float evaluate_bezier_x(float t, float x0, float x1, float x2) {
    float mt = 1.0 - t;
    return mt * mt * x0 + 2.0 * mt * t * x1 + t * t * x2;
}

void main() {
    Glyph g = glyphs[vInstanceID];
    vec2 position = vec2(relativePos.x/u_scale, relativePos.y/u_scale);
    
    if (!is_inbound(position.x, position.y, float(g.xmin), float(g.ymin), float(g.xmax), float(g.ymax))) {
        discard;
        return;
    }
    
    int winding_number = 0;
    uint pointBase = g.points_offset;
    
    for (int i = 0; i < g.num_contours; i++) {
        uint contourIdx = g.contours_offset + uint(i);
        uint startIdx = i == 0 ? 0 : contours[g.contours_offset + uint(i-1)] + 1;
        uint endIdx = contours[contourIdx];
        
        if (startIdx >= endIdx) continue;
        
        for (uint j = startIdx; j <= endIdx; j++) {
            uint currIdx = j;
            uint nextIdx = (j == endIdx) ? startIdx : j + 1;
            
            Point p1 = points[pointBase + currIdx];
            Point p2 = points[pointBase + nextIdx];
            
            if (bit_set(p1.flag, 0) && bit_set(p2.flag, 0)) {
                if (((p1.y <= position.y && p2.y > position.y) || 
                     (p1.y > position.y && p2.y <= position.y)) &&
                    (position.x < (p2.x - p1.x) * (position.y - p1.y) / float(p2.y - p1.y) + p1.x)) {
                    winding_number++;
                }
                continue;
            }
            
            if (currIdx != endIdx && !bit_set(p1.flag, 0) && bit_set(p2.flag, 0)) {
                uint prevIdx = (currIdx == startIdx) ? endIdx : currIdx - 1;
                Point p0 = points[pointBase + prevIdx];
                
                if (!bit_set(p0.flag, 0)) continue;
                
                float y0 = float(p0.y);
                float y1 = float(p1.y);
                float y2 = float(p2.y);
                
                if ((y0 < position.y && y1 < position.y && y2 < position.y) || 
                    (y0 > position.y && y1 > position.y && y2 > position.y)) {
                    continue;
                }
                
                // Solve for t where bezier(t).y = position.y
                float a = y2 - 2.0 * y1 + y0;
                float b = 2.0 * (y1 - y0);
                float c = y0 - position.y;
                
                // Handle nearly flat curves
                if (abs(a) < 0.0001) {
                    if (abs(b) > 0.0001) {
                        float t = -c / b;
                        if (t >= 0.0 && t <= 1.0) {
                            float x0 = float(p0.x);
                            float x1 = float(p1.x);
                            float x2 = float(p2.x);
                            float bezier_x = evaluate_bezier_x(t, x0, x1, x2);
                            if (position.x < bezier_x) winding_number++;
                        }
                    }
                    continue;
                }
                
                float discriminant = b * b - 4.0 * a * c;
                if (discriminant >= 0.0) {
                    float x0 = float(p0.x);
                    float x1 = float(p1.x);
                    float x2 = float(p2.x);
                    
                    float t1 = (-b + sqrt(discriminant)) / (2.0 * a);
                    if (t1 >= 0.0 && t1 <= 1.0) {
                        float bezier_x = evaluate_bezier_x(t1, x0, x1, x2);
                        if (position.x < bezier_x) winding_number++;
                    }
                    
                    float t2 = (-b - sqrt(discriminant)) / (2.0 * a);
                    if (t2 >= 0.0 && t2 <= 1.0) {
                        float bezier_x = evaluate_bezier_x(t2, x0, x1, x2);
                        if (position.x < bezier_x) winding_number++;
                    }
                }
            }
        }
    }
    
    if (winding_number % 2 == 0) {
        discard;
    } else {
        FragColor = vec4(1.0);
    }
}