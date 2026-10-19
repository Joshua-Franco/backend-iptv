# IPTV Peru API

Backend en **Rust** con **Axum 0.8**, **SQLx**, **Tokio** y **PostgreSQL** que sirve los canales de televisión abierta del Perú (TDT Lima), catálogo IPTV internacional y enlaces M3U/HLS personalizados.

## Estructura

```
backend-iptv/
├── Cargo.toml
├── src/
│   ├── main.rs            # Binario principal (iptv-peru-api)
│   ├── lib.rs             # Construcción del router y middleware
│   ├── error.rs           # Manejo de errores y respuestas JSON
│   ├── middleware.rs      # CORS, normalización de rutas, Vary, 405
│   ├── validate.rs        # Validaciones de query y body
│   ├── config/            # Configuración (.env)
│   ├── database/          # Conexión a PostgreSQL, migraciones y seed
│   ├── models/            # Modelos FromRow (SQLx)
│   ├── schemas/           # DTOs de salida y entrada
│   ├── routes/            # Endpoints de la API
│   └── services/          # Canales, HLS proxy, imágenes, M3U parser, paleta, webfetch
├── scripts/
│   ├── import_iptv_org.rs # Importador del catálogo iptv-org
│   ├── seed.rs            # Sembrado y actualización de canales base
│   └── test_db.rs         # Verificación de conexión a PostgreSQL
└── static/logos/          # Logos oficiales de los canales
```

## Requisitos

- Rust 1.80+ (edición 2021)
- PostgreSQL (Neon o local)

## Configuración (.env)

Crea o conserva el archivo `.env` en la raíz del proyecto:

```env
PROJECT_NAME=IPTV Peru API
API_PREFIX=/api
DATABASE_URL=postgresql://usuario:contrasena@host/basedatos?sslmode=require
HOST=0.0.0.0
PORT=8000
CORS_ORIGINS=*
```

## Ejecución

Compilar y ejecutar en modo desarrollo:

```powershell
cargo run
```

Compilar y ejecutar en modo optimizado (producción):

```powershell
cargo run --release
```

Al arrancar, las tablas e índices se crean automáticamente y si la base de datos está vacía se insertan los 12 canales base.

## Scripts auxiliares

- Probar conexión y listar tablas:
  ```powershell
  cargo run --bin test_db
  ```
- Sembrar / actualizar canales base:
  ```powershell
  cargo run --bin seed
  ```
- Importar catálogo iptv-org:
  ```powershell
  cargo run --bin import_iptv_org -- --refresh
  ```

## Endpoints

| Método | Ruta | Descripción |
| --- | --- | --- |
| GET | `/` | Información general de la API |
| GET | `/api/health` | Estado del servidor y base de datos |
| GET | `/api/channels` | Lista canales base y personalizados |
| GET | `/api/channels/categories` | Categorías con cantidad de canales |
| GET | `/api/channels/{channel_id}` | Detalle de un canal |
| GET | `/api/channels/by-uuid/{uuid}` | Canal por UUID |
| PATCH | `/api/channels/{channel_id}/stream` | Actualiza la señal de un canal |
| GET | `/api/iptv/categories` | Categorías del catálogo iptv-org |
| GET | `/api/iptv/categories/{code}/channels` | Canales por categoría iptv-org |
| GET | `/api/iptv/catalog` | Catálogo completo con búsqueda y paginación |
| GET | `/api/iptv/channels?ids=...` | Canales iptv por lista de IDs/UUIDs |
| GET | `/api/iptv/channels/{id}/logo` | Proxy y cache de logos iptv |
| GET | `/api/custom/sources` | Fuentes M3U personalizadas |
| POST | `/api/custom/sources` | Agrega lista M3U o canal suelto |
| DELETE | `/api/custom/sources/{id}` | Elimina fuente personalizada |
| GET | `/api/custom/channels/{id}/logo` | Proxy y cache de logos personalizados |
| GET | `/api/stream/{id}/playlist.m3u8` | Proxy HLS con reescritura de variantes |
| GET | `/api/stream/{id}/variant` | Proxy de playlist variante |
| GET | `/api/stream/{id}/asset` | Streaming de segmentos multimedia |
| GET | `/static/logos/{archivo}` | Logos estáticos oficiales |
