FROM ghcr.io/cirruslabs/flutter AS frontend

WORKDIR /flutter

COPY frontend/pubspec.yaml .
COPY frontend/web web
RUN flutter pub get
COPY frontend/lib lib
COPY frontend/assets assets
RUN flutter build web

FROM nginx:alpine

COPY nginx /etc/nginx/conf.d
COPY static /static
COPY --from=frontend /flutter/build/web /flutter
