//! Los recursos de Twingate, para el applet de red.
//!
//! Twingate no habla por NetworkManager —de ahí que el panel lo detecte por su
//! interfaz `tun`— y no expone ni D-Bus ni un socket documentado: lo que hay es
//! su cliente de línea de comandos, que le pregunta al demonio local y contesta
//! en 30 milisegundos. Por eso se lo llama en el momento en que se abre el
//! applet, sin cache ni sondeo de fondo.
//!
//! Lo que se lee es una tabla de columnas separadas por tabulaciones, con un
//! encabezado por grupo. El formato no está documentado como interfaz estable,
//! así que el estado de autorización se guarda **también en crudo**: si Twingate
//! agrega un estado que no conocemos, el applet muestra ese texto tal cual en
//! vez de decir algo equivocado.

use std::time::Duration;

use serde::Serialize;
use tokio::process::Command;

const BINARY: &str = "twingate";

/// El comando habla con un demonio local, así que esto no es «cuánto tarda la
/// red» sino «cuánto se espera antes de dar por colgado al demonio».
const TIMEOUT: Duration = Duration::from_secs(5);

/// Lo que contesta Twingate cuando no hay sesión: no es un error, es el estado
/// normal de una máquina desconectada.
const NO_CONECTADO: &str = "Twingate must be connected to display available resources.";

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct TwingateResource {
    pub name: String,
    pub address: String,
    /// El nombre corto con el que se lo puede llamar, cuando tiene uno.
    pub alias: Option<String>,
    /// `MAIN`, `KUBERNETES` o `BACKGROUND`, tal como los agrupa Twingate.
    pub group: String,
    /// El texto de estado sin interpretar, para poder mostrarlo si no lo
    /// entendemos.
    pub status: String,
    /// Está listo para usar: o no necesita autenticación, o ya está autorizado.
    pub usable: bool,
    /// Hace falta autorizarlo para poder entrar.
    pub needs_auth: bool,
    /// Cuánto falta para que se venza la autorización, en las palabras de
    /// Twingate («2 days», «over a week», «under 1 minute»).
    pub expires_in: Option<String>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct TwingateInfo {
    /// Si el cliente está instalado. Sin esto, el applet no muestra la sección.
    pub installed: bool,
    /// `online`, `offline`… lo que conteste `twingate status`, en crudo.
    pub status: String,
    pub connected: bool,
    pub resources: Vec<TwingateResource>,
}

fn instalado() -> bool {
    // Sin PATH heredado del entorno gráfico esto podría fallar aunque el
    // cliente esté; `which` sigue las mismas reglas que el `Command` de abajo.
    std::path::Path::new("/usr/bin/twingate").exists()
        || std::env::var_os("PATH")
            .map(|rutas| {
                std::env::split_paths(&rutas).any(|dir| dir.join(BINARY).exists())
            })
            .unwrap_or(false)
}

async fn correr(argumentos: &[&str]) -> Result<String, String> {
    // `kill_on_drop` no es cosmético: cuando el temporizador cancela el futuro,
    // el proceso hijo sigue corriendo por su cuenta y nadie lo espera nunca. Con
    // esto, el cliente que se colgó se muere junto con el intento.
    let futuro = Command::new(BINARY)
        .args(argumentos)
        .kill_on_drop(true)
        .output();

    let salida = tokio::time::timeout(TIMEOUT, futuro)
        .await
        .map_err(|_| format!("`{BINARY} {}` no contestó", argumentos.join(" ")))?
        .map_err(|error| format!("no se pudo ejecutar {BINARY}: {error}"))?;

    // El cliente escribe en la salida estándar incluso cuando el estado es
    // «desconectado», así que el código de salida no alcanza para decidir.
    Ok(String::from_utf8_lossy(&salida.stdout).to_string())
}

/// El grupo al que pertenecen las filas que siguen, si la línea es un
/// encabezado de grupo.
///
/// Los encabezados son la única línea sin tabulaciones: «MAIN RESOURCES»,
/// «KUBERNETES RESOURCES», «BACKGROUND RESOURCES».
fn grupo_de(linea: &str) -> Option<String> {
    let limpio = linea.trim();

    if limpio.is_empty() || linea.contains('\t') {
        return None;
    }

    Some(
        limpio
            .strip_suffix(" RESOURCES")
            .unwrap_or(limpio)
            .to_string(),
    )
}

/// Interpreta el estado de autorización.
///
/// Devuelve `(usable, needs_auth, expires_in)`. Los tres casos conocidos son:
/// vacío —el recurso no pide autenticación—, «Not authenticated» —hay que
/// autorizarlo— y «Auth expires in …» —autorizado, con lo que falta—. Cualquier
/// otra cosa se deja sin interpretar: se muestra el texto y no se ofrece un
/// botón que no sabemos si corresponde.
fn interpretar_estado(estado: &str) -> (bool, bool, Option<String>) {
    let estado = estado.trim();

    if estado.is_empty() {
        return (true, false, None);
    }

    if let Some(resto) = estado.strip_prefix("Auth expires in ") {
        return (true, false, Some(resto.trim().to_string()));
    }

    if estado.eq_ignore_ascii_case("Not authenticated") {
        return (false, true, None);
    }

    (false, false, None)
}

pub fn parse_resources(salida: &str) -> Vec<TwingateResource> {
    let mut recursos = Vec::new();
    let mut grupo = String::from("MAIN");

    for linea in salida.lines() {
        if linea.trim() == NO_CONECTADO {
            return Vec::new();
        }

        if let Some(nuevo) = grupo_de(linea) {
            grupo = nuevo;
            continue;
        }

        let columnas: Vec<&str> = linea.split('\t').map(str::trim).collect();

        // El encabezado de la tabla se repite por grupo.
        if columnas.first() == Some(&"RESOURCE NAME") {
            continue;
        }

        // Nombre, dirección, alias y estado. El estado puede venir vacío, así
        // que se acepta la fila con tres columnas y estado ausente.
        if columnas.len() < 3 || columnas[0].is_empty() {
            continue;
        }

        let estado = columnas.get(3).copied().unwrap_or("");
        let (usable, needs_auth, expires_in) = interpretar_estado(estado);

        recursos.push(TwingateResource {
            name: columnas[0].to_string(),
            address: columnas[1].to_string(),
            alias: match columnas[2] {
                "" | "-" => None,
                alias => Some(alias.to_string()),
            },
            group: grupo.clone(),
            status: estado.to_string(),
            usable,
            needs_auth,
            expires_in,
        });
    }

    recursos
}

#[tauri::command]
pub async fn twingate_info() -> TwingateInfo {
    if !instalado() {
        return TwingateInfo {
            installed: false,
            status: String::new(),
            connected: false,
            resources: Vec::new(),
        };
    }

    let status = correr(&["status"]).await.unwrap_or_default().trim().to_string();
    let connected = status.eq_ignore_ascii_case("online");

    // Sin sesión, pedir los recursos devuelve una frase y no una tabla: se evita
    // el segundo llamado.
    let resources = if connected {
        match correr(&["resources", "-d"]).await {
            Ok(salida) => parse_resources(&salida),
            Err(error) => {
                eprintln!("[twingate] {error}");
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    TwingateInfo {
        installed: true,
        status,
        connected,
        resources,
    }
}

/// Un nombre de recurso es lo que imprime `twingate resources`.
///
/// El argumento no pasa por una shell —va como un elemento propio de `argv`—
/// así que esto no es una barrera de inyección, sino la forma de no arrastrar
/// hasta el cliente algo que claramente no es un nombre de recurso.
fn es_nombre_de_recurso(valor: &str) -> bool {
    // Que no empiece con guion no es cosmético: `twingate auth --print-commands`
    // trataría el argumento como una opción del cliente en vez de como el
    // recurso a autorizar.
    !valor.is_empty()
        && !valor.starts_with('-')
        && valor.len() <= 128
        && valor
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Arranca la autorización de un recurso bloqueado.
///
/// El cliente abre el navegador y espera la confirmación del otro lado, así que
/// esto no espera el final: deja el proceso corriendo y vuelve. El applet
/// vuelve a preguntar el estado cuando la persona lo abre de nuevo.
#[tauri::command]
pub async fn twingate_authorize(resource: String) -> Result<(), String> {
    if !es_nombre_de_recurso(&resource) {
        return Err(format!("«{resource}» no parece un nombre de recurso"));
    }

    Command::new(BINARY)
        .args(["auth", &resource])
        .spawn()
        .map_err(|error| format!("no se pudo pedir la autorización: {error}"))?;

    Ok(())
}

/// Abre o esconde el applet de Twingate.
#[tauri::command]
pub fn toggle_twingate_applet(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::{async_runtime::spawn, Emitter, Manager};

    if let Some(ventana) = app.get_webview_window("applet_twingate") {
        if ventana.is_visible().unwrap_or(false) {
            let _ = ventana.hide();
        } else {
            // `window-shown` es lo que hace que la vista vuelva a preguntar:
            // esconder no destruye el webview, así que Vue no se monta de nuevo.
            let _ = ventana.emit("window-shown", ());
            let _ = ventana.show();
            let _ = ventana.set_focus();
        }

        return Ok(());
    }

    spawn(async move {
        if let Err(error) = crate::windows_apps::create_applet_twingate_window(app).await {
            eprintln!("[twingate] No se pudo abrir el applet: {error}");
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Una salida real, recortada: dos grupos, un alias ausente, un estado
    /// vacío y uno sin autenticar.
    const SALIDA: &str = "MAIN RESOURCES\nRESOURCE NAME\tADDRESS\tALIAS\tAUTH STATUS\nack-service\tack-service.private.svc\tack-service.twingate\tAuth expires in 2 days\nprod-database\tprod.rds.amazonaws.com\t-\t\nprod-jenkins\t10.0.0.27\tjenkins.pickitlabs.ar\tNot authenticated\n\nKUBERNETES RESOURCES\nRESOURCE NAME\tADDRESS\tALIAS\tAUTH STATUS\ntest-cluster\tkubernetes.default.svc\tpickit-test-cluster.int\tAuth expires in over a week\n";

    #[test]
    fn lee_las_filas_de_los_dos_grupos() {
        let recursos = parse_resources(SALIDA);

        assert_eq!(recursos.len(), 4);
        assert_eq!(recursos[0].name, "ack-service");
        assert_eq!(recursos[0].group, "MAIN");
        assert_eq!(recursos[3].group, "KUBERNETES");
    }

    #[test]
    fn el_encabezado_de_cada_grupo_no_es_un_recurso() {
        assert!(
            !parse_resources(SALIDA)
                .iter()
                .any(|r| r.name == "RESOURCE NAME"),
            "la fila de títulos se cuela como recurso"
        );
    }

    #[test]
    fn un_recurso_autorizado_dice_cuanto_le_falta() {
        let recursos = parse_resources(SALIDA);

        assert!(recursos[0].usable);
        assert!(!recursos[0].needs_auth);
        assert_eq!(recursos[0].expires_in.as_deref(), Some("2 days"));
        assert_eq!(recursos[3].expires_in.as_deref(), Some("over a week"));
    }

    #[test]
    fn un_recurso_sin_autenticar_se_puede_autorizar() {
        let jenkins = parse_resources(SALIDA)
            .into_iter()
            .find(|r| r.name == "prod-jenkins")
            .expect("está en la muestra");

        assert!(jenkins.needs_auth);
        assert!(!jenkins.usable);
        assert_eq!(jenkins.expires_in, None);
    }

    #[test]
    fn un_estado_vacio_es_un_recurso_que_no_pide_autenticacion() {
        let base = parse_resources(SALIDA)
            .into_iter()
            .find(|r| r.name == "prod-database")
            .expect("está en la muestra");

        assert!(base.usable, "no pide autenticación, así que se puede usar");
        assert!(!base.needs_auth);
        assert_eq!(base.alias, None, "el guion no es un alias");
    }

    #[test]
    fn un_estado_desconocido_se_muestra_sin_interpretar() {
        let (usable, needs_auth, expira) = interpretar_estado("Something new");

        assert!(!usable);
        assert!(
            !needs_auth,
            "no se ofrece autorizar por un estado que no conocemos"
        );
        assert_eq!(expira, None);
    }

    #[test]
    fn sin_sesion_no_hay_recursos() {
        assert!(parse_resources(NO_CONECTADO).is_empty());
    }

    #[test]
    fn una_salida_vacia_no_rompe() {
        assert!(parse_resources("").is_empty());
    }

    #[test]
    fn los_nombres_de_recurso_reales_pasan_el_filtro() {
        assert!(es_nombre_de_recurso("preprod-ms-retailer-integrations"));
        assert!(es_nombre_de_recurso("test-cluster"));
        assert!(es_nombre_de_recurso("prod-pickit-read-replica"));
    }

    #[test]
    fn lo_que_no_es_un_nombre_no_pasa() {
        assert!(!es_nombre_de_recurso(""));
        assert!(!es_nombre_de_recurso("dos palabras"));
        assert!(!es_nombre_de_recurso("--print-commands"), "una opción");
        assert!(!es_nombre_de_recurso("a;b"));
        assert!(!es_nombre_de_recurso(&"a".repeat(129)));
    }
}
