# Built from web/ (see docker-compose.yml) so the shared ui/ is in reach.
FROM node:20-alpine AS build
WORKDIR /app
COPY ui ./ui
COPY accounts/package*.json ./accounts/
RUN cd accounts && npm install
COPY accounts ./accounts
RUN cd accounts && npm run build

FROM nginx:alpine
COPY --from=build /app/accounts/dist /usr/share/nginx/html
COPY accounts/nginx.conf /etc/nginx/conf.d/default.conf
