# Built from the repo root (see docker-compose.yml) so the shared ui/ is in reach.
FROM node:20-alpine AS build
WORKDIR /app
COPY ui ./ui
COPY cloud/package*.json ./cloud/
RUN cd cloud && npm install
COPY cloud ./cloud
RUN cd cloud && npm run build

FROM nginx:alpine
COPY --from=build /app/cloud/dist /usr/share/nginx/html
COPY cloud/nginx.conf /etc/nginx/conf.d/default.conf
