# 3D Cave Generation

This document outlines the design and implementation of 3D cave generation in Ruxel.

## Multi-layer Noise System

Ruxel uses a multi-layered approach to generate realistic and interesting cave systems using Perlin noise. The system relies on two primary noise layers that represent different types of underground structures.

### 1. "Cheese" Caverns
- **Noise Type:** `Fbm<Perlin>`
- **Characteristics:** Medium frequency (0.015), 3 octaves.
- **Purpose:** Creates large, blobby, cavernous rooms.
- **Evaluation:** Evaluated by thresholding the noise value (`noise > threshold`).

### 2. "Spaghetti" Tunnels
- **Noise Type:** `Fbm<Perlin>`
- **Characteristics:** Low frequency (0.01), 2 octaves.
- **Purpose:** Creates winding, tubular tunnels that connect larger caverns.
- **Evaluation:** Evaluated by taking the absolute value of the noise and thresholding near zero (`abs(noise) < threshold`). This naturally creates 3D ridges/tubes.

## Combining and Attenuation

A block is considered a cave (air block) if it satisfies **either** the Cheese cavern condition **or** the Spaghetti tunnel condition.

To prevent caves from breaking through the surface too frequently or unnaturally, a **depth attenuation** factor is applied:
- Caves are completely disabled within 10 blocks of the surface.
- Between a depth of 10 and 50 blocks, the threshold for generating a cave becomes stricter (closer to the surface, fewer caves).
- Below a depth of 50 blocks, caves generate at their base frequencies and sizes.

This system is globally applied across all biomes, though future iterations could modulate these thresholds based on the primary surface biome (e.g., larger caverns under mountains).
