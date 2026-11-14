# The gateway: Caddy, with every web app built into it. Built from web/ (see docker-compose.yml).
# Each app builds in its own stage, so changing one only rebuilds that one; Caddy then serves the
# built files itself, from /srv/<app>.

FROM node:20-alpine AS login
WORKDIR /app
COPY ui ./ui
COPY login/package*.json ./login/
RUN cd login && npm ci --no-audit --no-fund
COPY login ./login
RUN cd login && npm run build

FROM node:20-alpine AS accounts
WORKDIR /app
COPY ui ./ui
COPY accounts/package*.json ./accounts/
RUN cd accounts && npm ci --no-audit --no-fund
COPY accounts ./accounts
RUN cd accounts && npm run build

FROM node:20-alpine AS cloud
WORKDIR /app
COPY ui ./ui
COPY cloud/package*.json ./cloud/
RUN cd cloud && npm ci --no-audit --no-fund
COPY cloud ./cloud
RUN cd cloud && npm run build

FROM node:20-alpine AS home
WORKDIR /app
COPY ui ./ui
COPY home/package*.json ./home/
RUN cd home && npm ci --no-audit --no-fund
COPY home ./home
RUN cd home && npm run build

FROM node:20-alpine AS store
WORKDIR /app
COPY ui ./ui
COPY store/package*.json ./store/
RUN cd store && npm ci --no-audit --no-fund
COPY store ./store
RUN cd store && npm run build

FROM node:20-alpine AS photos
WORKDIR /app
COPY ui ./ui
COPY photos/package*.json ./photos/
RUN cd photos && npm ci --no-audit --no-fund
COPY photos ./photos
RUN cd photos && npm run build

FROM node:20-alpine AS share
WORKDIR /app
COPY ui ./ui
COPY share/package*.json ./share/
RUN cd share && npm ci --no-audit --no-fund
COPY share ./share
RUN cd share && npm run build

FROM caddy:2
COPY --from=login /app/login/dist /srv/login
COPY --from=accounts /app/accounts/dist /srv/accounts
COPY --from=cloud /app/cloud/dist /srv/cloud
COPY --from=home /app/home/dist /srv/home
COPY --from=store /app/store/dist /srv/store
COPY --from=photos /app/photos/dist /srv/photos
COPY --from=share /app/share/dist /srv/share
