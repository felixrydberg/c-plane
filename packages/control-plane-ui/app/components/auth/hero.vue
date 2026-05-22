<script setup lang="ts">
import { ChromaFlow, Dither, FilmGrain, Shader, StudioBackground } from 'shaders/vue'
</script>

<template>
  <div class="absolute inset-0 overflow-hidden rounded-r-4xl bg-black">
    <div class="absolute inset-0 opacity-90">
      <Shader class="size-full" disable-telemetry>
        <StudioBackground
          color="#121727"
          key-color="#1f2a44"
          fill-color="#182136"
          back-color="#06080f"
          :brightness="0.3"
          :ambient-intensity="0.3"
          :fill-intensity="0.28"
          :fill-angle="24"
          :back-intensity="0.24"
          :vignette="0.58"
        />
        <ChromaFlow
          base-color="#1e3a8a"
          up-color="#334155"
          down-color="#0f172a"
          left-color="#2563eb"
          right-color="#1d4ed8"
          :intensity="0.58"
          :radius="2.4"
          :momentum="22"
        />
        <FilmGrain
          :strength="0.1"
          :opacity="0.2"
        />
        <Dither
          blend-mode="overlay"
          :pixel-size="2"
        />
      </Shader>
    </div>
  </div>
</template>
