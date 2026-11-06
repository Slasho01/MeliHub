//! Planes de suscripción (3 niveles). Los límites se aplican de forma suave:
//! `-1` significa ilimitado. Por defecto todos quedan en `business` (sin límite),
//! así que hoy no limita a nadie; el mecanismo queda listo para comercializar.

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Plan {
    pub id: &'static str,
    pub nombre: &'static str,
    /// -1 = ilimitado
    pub max_cuentas: i64,
    pub max_productos: i64,
    pub max_plataformas: i64,
    /// Precio mensual (CLP)
    pub precio_mensual: i64,
    /// Precio anual (CLP) — 2 meses gratis respecto al mensual
    pub precio_anual: i64,
}

/// Días de prueba gratis en la primera suscripción a un plan de pago.
pub const TRIAL_DIAS: i64 = 14;

pub const PLANES: &[Plan] = &[
    Plan { id: "free", nombre: "Free", max_cuentas: 1, max_productos: 50, max_plataformas: 1, precio_mensual: 0, precio_anual: 0 },
    Plan { id: "pro", nombre: "Pro", max_cuentas: 3, max_productos: 1000, max_plataformas: 3, precio_mensual: 9990, precio_anual: 99900 },
    Plan { id: "business", nombre: "Business", max_cuentas: -1, max_productos: -1, max_plataformas: -1, precio_mensual: 29990, precio_anual: 299900 },
];

/// Plan por id; si no se reconoce, cae al más alto (no bloquear por error de dato).
pub fn plan(id: &str) -> &'static Plan {
    PLANES.iter().find(|p| p.id == id).unwrap_or_else(|| PLANES.last().unwrap())
}

/// Precio del plan según el ciclo ("anual" → precio_anual; cualquier otro → mensual).
pub fn precio_de(plan: &Plan, ciclo: &str) -> i64 {
    if ciclo == "anual" {
        plan.precio_anual
    } else {
        plan.precio_mensual
    }
}

/// Normaliza el ciclo recibido a "mensual" | "anual".
pub fn ciclo_valido(ciclo: &str) -> &'static str {
    if ciclo == "anual" {
        "anual"
    } else {
        "mensual"
    }
}

/// ¿Se puede agregar uno más? (`limite` < 0 = ilimitado).
pub fn cabe(limite: i64, uso_actual: i64) -> bool {
    limite < 0 || uso_actual < limite
}
