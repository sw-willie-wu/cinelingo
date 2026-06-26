// 磨砂玻璃背景用：設定 Modal 開啟時對影片套的 2-pass 可分離高斯模糊。
// gpu-next 的 user shader（HOOK MAIN），在 GPU 算繪管線內執行 → 不破壞硬解、不吃 CPU。
// 由前端 setVideoBlur() 透過 glsl-shaders 屬性套用/移除。

//!DESC frosted-blur-h
//!HOOK MAIN
//!BIND HOOKED
vec4 hook() {
    vec4 sum = vec4(0.0);
    float wsum = 0.0;
    for (int i = -28; i <= 28; i++) {
        float w = exp(-float(i * i) / 288.0); // sigma ~12（vo=gpu 視覺較弱，加大半徑/sigma 補償）
        sum += HOOKED_texOff(vec2(float(i), 0.0)) * w;
        wsum += w;
    }
    return sum / wsum;
}

//!DESC frosted-blur-v
//!HOOK MAIN
//!BIND HOOKED
vec4 hook() {
    vec4 sum = vec4(0.0);
    float wsum = 0.0;
    for (int i = -28; i <= 28; i++) {
        float w = exp(-float(i * i) / 288.0); // sigma ~12（vo=gpu 視覺較弱，加大半徑/sigma 補償）
        sum += HOOKED_texOff(vec2(0.0, float(i))) * w;
        wsum += w;
    }
    return sum / wsum;
}
