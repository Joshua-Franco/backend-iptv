# IPTV Peru API

Backend en **FastAPI** con **PostgreSQL** que sirve los canales de television abierta del Peru (TDT Lima) junto con sus logos oficiales.

## Estructura

```
backend-iptv/
├── config/            # Configuracion (.env)
├── database/          # Conexion a PostgreSQL + seed de canales
├── models/            # Modelos ORM (SQLAlchemy 2.0)
├── schemas/           # Esquemas Pydantic
├── routes/            # Endpoints de la API
├── services/          # Logica de negocio
├── static/logos/      # Logos oficiales de los canales
├── scripts/           # Script de instalacion
├── main.py            # Aplicacion FastAPI
└── run.py             # Punto de entrada: arranca el servidor
```

## Requisitos

- Python 3.12+
- PostgreSQL 18 (o superior) corriendo en `localhost:5432`

## Instalacion

```powershell
.\scripts\setup.ps1
```

El script crea el entorno virtual, instala las dependencias y crea la base de datos `iptv` si no existe.

Configura tu contrasena de PostgreSQL en `.env` (no lo subas a git):

```
DATABASE_URL=postgresql+psycopg://postgres:TU_CONTRASENA@localhost:5432/iptv
```

## Ejecutar

```powershell
.\.venv\Scripts\python.exe run.py
```

El host, puerto y modo reload se configuran en `.env` (`HOST`, `PORT`, `RELOAD`).

Al arrancar, las tablas se crean solas y si la base de datos esta vacia se insertan los 12 canales (seed automatico).

Documentacion interactiva: http://localhost:8000/docs

## Endpoints

| Metodo | Ruta | Descripcion |
| --- | --- | --- |
| GET | `/api/health` | Estado del servidor y de la BD |
| GET | `/api/channels` | Lista todos los canales |
| GET | `/api/channels?category=Noticias` | Filtra por categoria |
| GET | `/api/channels?q=latina` | Busca por nombre/categoria |
| GET | `/api/channels/categories` | Categorias con su cantidad |
| GET | `/api/channels/{id}` | Un canal por id |
| GET | `/static/logos/{archivo}` | Logos de los canales |

## Canales incluidos

Willax (1), Latina (2), RPP TV (3), America Television (4), Panamericana (5), Exitosa (6), TV Peru (7), Canal N (8), ATV (9), La Tele (10), Viva TV (11), Global (13).

Los logos oficiales provienen de Wikimedia Commons (licencias libres) y el logo de America TV es un SVG propio.

## Sembrar datos manualmente

```powershell
python -m database.seed
```
