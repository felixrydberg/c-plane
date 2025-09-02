# FROM oven/bun:1.2.3-debian AS build
# RUN apt-get update && apt-get install -y python3-pip make g++ libc6-dev
# WORKDIR /usr/src/app/src

# FROM build as development
# COPY package*.json ./
# RUN --mount=type=bind,source=package.json,target=package.json
# RUN bun install
# CMD ["sh", "-c", "bun install && bun run dev"]

# FROM build as production-prepack
# COPY bun.lockb .
# COPY package.json .
# RUN bun install --frozen-lockfile
# COPY . .

# RUN bun run build
# RUN bun build .output/server/index.mjs --compile --outfile cli

# # We need to figure out how to run binary without massive commands
# # FROM alpine:3.21.0 as production
# FROM ubuntu:24.04 as production
# WORKDIR /usr/src/app/src
# COPY --from=production-prepack /usr/src/app/src/cli /usr/src/app/src/cli
# CMD ["./cli"]

FROM node:23-slim AS build
WORKDIR /usr/src/app/src

FROM build AS development
COPY package*.json ./
RUN --mount=type=bind,source=package.json,target=package.json \
--mount=type=bind,source=package-lock.json,target=package-lock.json
RUN npm install
CMD ["sh", "-c", "npm install && npm run dev"]

FROM build AS production-prepack
COPY package*.json ./
RUN npm install
COPY . .
RUN ["sh", "-c", "npm run build"]

# FROM node:21-alpine3.20 as production
FROM node:23-slim as production
COPY --from=production-prepack /usr/src/app/src/.output ./.output
CMD ["node", ".output/server/index.mjs"]
