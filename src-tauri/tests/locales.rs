//! Que los catálogos de idioma sirvan.
//!
//! El plugin de i18n los parsea en tiempo de ejecución y **paniquea** si no
//! puede, así que un error de sintaxis no se ve hasta que la aplicación no
//! arranca. Y una clave que falta en un idioma no falla: se muestra cruda, con
//! el nombre de la clave a la vista de la persona.
//!
//! Estos tests aparecieron al mover el bloque de permisos de `views.privacy` a
//! `views.onlineAccounts.permissions`: reindentar un bloque de veinte líneas a
//! mano en dos archivos es exactamente donde se pierde una clave.

use std::collections::BTreeSet;
use std::path::PathBuf;

fn catalogo(idioma: &str) -> serde_yaml::Value {
    let ruta = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("locales")
        .join(format!("{idioma}.yml"));
    let texto = std::fs::read_to_string(&ruta)
        .unwrap_or_else(|e| panic!("no se pudo leer {}: {e}", ruta.display()));
    serde_yaml::from_str(&texto)
        .unwrap_or_else(|e| panic!("{} no es YAML válido: {e}", ruta.display()))
}

/// Todas las claves, aplanadas con puntos, como las busca el plugin.
fn claves(valor: &serde_yaml::Value, prefijo: &str, salida: &mut BTreeSet<String>) {
    match valor {
        serde_yaml::Value::Mapping(mapa) => {
            for (clave, hijo) in mapa {
                let nombre = clave.as_str().unwrap_or_default();
                let completa = if prefijo.is_empty() {
                    nombre.to_string()
                } else {
                    format!("{prefijo}.{nombre}")
                };
                claves(hijo, &completa, salida);
            }
        }
        _ => {
            salida.insert(prefijo.to_string());
        }
    }
}

fn claves_de(idioma: &str) -> BTreeSet<String> {
    let mut salida = BTreeSet::new();
    claves(&catalogo(idioma), "", &mut salida);
    salida
}

#[test]
fn los_dos_idiomas_parsean_y_la_raiz_es_un_mapeo() {
    for idioma in ["es", "en"] {
        let raiz = catalogo(idioma);
        assert!(
            raiz.is_mapping(),
            "la raíz de {idioma}.yml tiene que ser un mapeo, no un valor suelto"
        );
    }
}

#[test]
fn los_dos_idiomas_tienen_las_mismas_claves() {
    let es = claves_de("es");
    let en = claves_de("en");

    let solo_es: Vec<_> = es.difference(&en).collect();
    let solo_en: Vec<_> = en.difference(&es).collect();

    assert!(
        solo_es.is_empty() && solo_en.is_empty(),
        "las claves no coinciden.\n  sólo en es: {solo_es:?}\n  sólo en en: {solo_en:?}"
    );
}

#[test]
fn ningun_texto_esta_vacio() {
    // Una clave vacía no es un texto faltante que se note: se muestra como nada,
    // y el control queda sin etiqueta.
    for idioma in ["es", "en"] {
        let raiz = catalogo(idioma);
        let mut vacias = Vec::new();
        let mut todas = BTreeSet::new();
        claves(&raiz, "", &mut todas);

        for clave in &todas {
            let mut actual = &raiz;
            for parte in clave.split('.') {
                actual = &actual[parte];
            }
            if actual.as_str().map(|s| s.trim().is_empty()).unwrap_or(false) {
                vacias.push(clave.clone());
            }
        }
        assert!(vacias.is_empty(), "textos vacíos en {idioma}.yml: {vacias:?}");
    }
}

#[test]
fn los_marcadores_de_interpolacion_coinciden() {
    // Un `{0}` que está en un idioma y no en el otro pierde el dato: el texto
    // sale sin el nombre del proveedor, sin el error, sin el número.
    let es = catalogo("es");
    let en = catalogo("en");
    let mut todas = BTreeSet::new();
    claves(&es, "", &mut todas);

    let marcadores = |v: &serde_yaml::Value| -> Vec<String> {
        let texto = v.as_str().unwrap_or_default();
        let mut encontrados: Vec<String> = texto
            .match_indices('{')
            .filter_map(|(i, _)| texto[i..].find('}').map(|j| texto[i..i + j + 1].to_string()))
            .collect();
        encontrados.sort();
        encontrados
    };

    for clave in &todas {
        let mut a = &es;
        let mut b = &en;
        for parte in clave.split('.') {
            a = &a[parte];
            b = &b[parte];
        }
        assert_eq!(
            marcadores(a),
            marcadores(b),
            "los marcadores de «{clave}» no coinciden entre idiomas"
        );
    }
}
