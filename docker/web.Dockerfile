FROM oven/bun:1 AS builder

WORKDIR /work

COPY package.json bun.lock ./
COPY apps/web/package.json apps/web/bun.lock ./apps/web/

RUN bun install --frozen-lockfile

COPY apps/web ./apps/web

WORKDIR /work/apps/web
RUN bun run build

FROM nginx:1.27-alpine AS runtime

COPY docker/web.nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /work/apps/web/dist /usr/share/nginx/html

EXPOSE 8080
