# Built from web/ (see docker-compose.yml) so the shared ui/ is in reach.
FROM node:20-alpine AS build
WORKDIR /app
COPY ui ./ui
COPY home/package*.json ./home/
RUN cd home && npm install
COPY home ./home
RUN cd home && npm run build

FROM nginx:alpine
COPY --from=build /app/home/dist /usr/share/nginx/html
COPY home/nginx.conf /etc/nginx/conf.d/default.conf
