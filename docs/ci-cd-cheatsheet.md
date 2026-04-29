# Fiche mémo CI/CD

En bref : GitHub Actions vérifie le code, Railway déploie le backend, le workflow CD vérifie la production, et les releases construisent les paquets desktop à partir des tags Git.

## Comment fonctionnent les tests

Les tests sont séparés en deux parties : frontend et backend.

Le frontend se trouve dans `chatroom` et se vérifie avec :

```bash
npm ci
npm run lint
npx next typegen
npx tsc --noEmit
npm test
npm run build
```

Ce que cela vérifie :

- `npm ci` installe les dépendances à partir de `chatroom/package-lock.json`;
- `npm run lint` lance ESLint;
- `npx next typegen` génère les types Next.js;
- `npx tsc --noEmit` vérifie TypeScript;
- `npm test` lance les tests Vitest dans `chatroom/lib`;
- `npm run build` vérifie le build production Next.js utilisé par Tauri.

Le backend se trouve dans `backend/rtc_backend` et se vérifie avec :

```bash
cargo fmt --all --check
cargo check --locked
cargo test --locked
```

Dans le CI, les tests backend utilisent un PostgreSQL `16` exposé sur le port `5433`. Les tests lisent `TEST_DATABASE_URL` ou `DATABASE_URL`, exécutent les migrations, puis vérifient les repositories, les services, l'auth/JWT, les WebSockets et les principaux flows API.

## Comment voir la test coverage

La couverture backend se mesure avec `cargo-llvm-cov`.

Installation une seule fois :

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
```

Lancer le rapport :

```bash
cd backend/rtc_backend
cargo llvm-cov --locked --fail-under-lines 70 --lcov --output-path coverage/lcov.info
cargo llvm-cov report --summary-only
```

Résultat utile :

- le terminal affiche un résumé de coverage;
- le fichier LCOV est généré dans `backend/rtc_backend/coverage/lcov.info`;
- `--fail-under-lines 70` fait échouer la commande si la couverture ligne passe sous `70%`.

Côté frontend, les tests Vitest existent, mais aucun script de coverage n'est actuellement configuré dans `chatroom/package.json`.

## Comment fonctionne le CI

Le CI est défini dans `.github/workflows/ci.yml`.

Il se lance :

- à chaque `push` sur n'importe quelle branche;
- sur les `pull_request` vers `main` ou `master`.

Il contient deux jobs indépendants :

- `Frontend` pour l'application Next.js/Tauri;
- `Backend` pour le backend Rust.

Si une étape échoue, le CI devient rouge. Pour une même branche, un ancien run CI est annulé quand un nouveau push arrive.

## Comment fonctionne le CD

Le CD est défini dans `.github/workflows/cd.yml`.

Il se lance :

- automatiquement après un CI réussi sur `main`;
- manuellement avec `workflow_dispatch`.

Important : ce workflow ne déploie pas directement le backend. Il fait seulement un smoke test de la production :

1. il attend `45` secondes;
2. il appelle `https://orbitotterchat-production.up.railway.app/`;
3. il réessaie jusqu'à `20` fois avec `15` secondes entre chaque tentative;
4. il réussit uniquement si le backend répond avec HTTP `200`.

Vérification locale équivalente :

```bash
curl -i https://orbitotterchat-production.up.railway.app/
```

## Comment fonctionne le déploiement

Le backend de production est déployé par Railway.

Le fichier `railway.toml` indique à Railway :

- d'utiliser un build Dockerfile;
- d'utiliser `backend/rtc_backend/Dockerfile`;
- de vérifier la santé du service sur `/`;
- de surveiller les changements dans `backend/rtc_backend/**`, `.dockerignore` et `railway.toml`.

Le Dockerfile compile le backend Rust, installe `diesel_cli`, copie les migrations et lance `/app/entrypoint.sh`. Au démarrage, l'entrypoint attend la base de données, lance `diesel migration run`, puis démarre `rtc_backend`.

Le backend écoute sur `PORT` si Railway le fournit, sinon sur `3000`.

## Configuration Railway

Dans Railway, le projet doit contenir au minimum :

- un service PostgreSQL;
- un service backend construit depuis ce repository;
- un volume persistant monté dans le backend pour les uploads.

Variables d'environnement importantes pour le backend :

```text
DATABASE_URL=<URL PostgreSQL Railway>
JWT_SECRET=<secret fort de production>
RUST_LOG=info
```

`DATABASE_URL` doit pointer vers la base PostgreSQL Railway. C'est cette variable que Diesel utilise pour exécuter les migrations et que le backend utilise pour ouvrir ses connexions SQL.

Le volume des fichiers uploadés doit être monté sur :

```text
/app/uploads
```

Pourquoi c'est important :

- les avatars sont écrits dans `uploads/avatars`;
- les pièces jointes sont écrites dans `uploads/attachments`;
- l'application sert les fichiers via `/uploads`;
- sans volume persistant, les fichiers peuvent disparaître au redémarrage ou au redéploiement du conteneur.

Pour un déploiement self-hosted, `deploy/docker-compose.prod.yml` montre la même logique avec `db`, `backend`, `frontend`, un volume PostgreSQL et un volume `backend_uploads:/app/uploads`.

## Comment fonctionnent les releases

Les releases sont définies dans `.github/workflows/release.yml`.

Le workflow Release se lance :

- quand un tag Git de forme `v*` est poussé;
- manuellement, en donnant un tag existant.

La release construit les paquets Linux desktop via Tauri :

```bash
./scripts/build-desktop-release.sh
```

Ce script entre dans `chatroom`, installe les dépendances et lance :

```bash
npm run tauri -- build
```

Les fichiers publiés dans GitHub Release sont :

- `.deb`;
- `.rpm`;
- `.AppImage`.

Le client desktop est construit avec l'URL backend de production :

```text
NEXT_PUBLIC_API_URL=https://orbitotterchat-production.up.railway.app
```

## Comment fonctionnent les tags

Un tag de forme `v*` déclenche une release. Exemple :

```bash
git tag -a v0.1.1 -m "OrbitOtterChat v0.1.1"
git push origin v0.1.1
```

Après le push du tag, GitHub Actions :

1. lance le workflow `Release`;
2. checkout exactement ce tag;
3. construit les paquets desktop;
4. crée une GitHub Release nommée `OrbitOtterChat <tag>`;
5. upload les artefacts.

Si la release existe déjà pour ce tag, le workflow ré-upload les assets avec `--clobber`.

Vérifier une release :

```bash
gh release view v0.1.1 --json url,tagName,assets
```
