# Built from the repo root (see docker-compose.yml) so the shared ui/ is in reach.
FROM node:20-alpine AS build
WORKDIR /app
COPY ui ./ui
COPY login/package*.json ./login/
RUN cd login && npm install
COPY login ./login
RUN cd login && npm run build

FROM nginx:alpine
COPY --from=build /app/login/dist /usr/share/nginx/html
COPY login/nginx.conf /etc/nginx/conf.d/default.conf
