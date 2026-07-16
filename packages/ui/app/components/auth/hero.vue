<script setup lang="ts">
import { ChromaFlow, Dither, FilmGrain, Shader, StudioBackground } from 'shaders/vue'
</script>

<template>
  <div class="absolute inset-0 overflow-hidden bg-[#0b0b0b]">
    <div class="absolute inset-0 opacity-60">
      <Shader class="size-full" disable-telemetry>
        <StudioBackground
          color="#212121"
          key-color="#3c4758"
          fill-color="#353535"
          back-color="#0b0b0b"
          :brightness="0.22"
          :ambient-intensity="0.2"
          :fill-intensity="0.2"
          :fill-angle="24"
          :back-intensity="0.2"
          :vignette="0.72"
        />
        <ChromaFlow
          base-color="#212121"
          up-color="#3c4758"
          down-color="#0b0b0b"
          left-color="#353535"
          right-color="#505b6c"
          :intensity="0.36"
          :radius="2.4"
          :momentum="22"
        />
        <FilmGrain
          :strength="0.08"
          :opacity="0.14"
        />
        <Dither
          blend-mode="overlay"
          :pixel-size="2"
        />
      </Shader>
    </div>
  </div>
</template>
