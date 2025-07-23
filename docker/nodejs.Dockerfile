FROM node:20-bookworm

ENV COREPACK_ENABLE_DOWNLOAD_PROMPT=0

WORKDIR /app

COPY .yarnrc.yml package.json yarn.lock ./
COPY .yarn/ ./.yarn/
COPY client/nodejs/package.json ./client/nodejs/
COPY testing/nodejs/package.json ./testing/nodejs/
COPY bitcoin/nodejs/package.json ./bitcoin/nodejs/

RUN corepack enable \
 && corepack prepare yarn@stable --activate

RUN yarn install

COPY client/nodejs/ ./client/nodejs/
COPY testing/nodejs/ ./testing/nodejs/
COPY bitcoin/nodejs/ ./bitcoin/nodejs/

RUN yarn tsc

CMD ["node", "client/nodejs/lib/cli.js"]
