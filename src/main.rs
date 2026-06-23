use std::env; 
use std::io::{self, BufRead};
use serde::Deserialize;
use serde_json::json;

// Estructura para la respuesta de la API de AEMET
#[derive(Deserialize, Debug)]
struct AemetResponse {
    datos: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("AEMET_API_KEY")
        .expect("Por favor, configura la variable de entorno AEMET_API_KEY");

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut line = String::new();

    // Bucle principal para escuchar peticiones de la IA (vía stdin)
    while handle.read_line(&mut line)? > 0 {
        let request: serde_json::Value = match serde_json::from_str(&line) {
            Ok(val) => val,
            Err(_) => {
                line.clear();
                continue;
            }
        };

        let id = &request["id"];
        let method = request["method"].as_str().unwrap_or("");

        match method {
            "initialize" => {
                // Responder al handshake de inicialización de MCP
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {
                            "tools": {}
                        },
                        "serverInfo": {
                            "name": "mcp_temp",
                            "version": "0.1.0"
                        }
                    }
                });
                println!("{}", response.to_string());
            }
            "tools/list" => {
                // Listar las herramientas disponibles para el modelo
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": [
                            {
                                "name": "get_sevilla_weather",
                                "description": "Obtiene los datos climatológicos actuales y la predicción para Sevilla desde la API de AEMET.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {}
                                }
                            }
                        ]
                    }
                });
                println!("{}", response.to_string());
            }
            "tools/call" => {
                let tool_name = request["params"]["name"].as_str().unwrap_or("");
                
                if tool_name == "get_sevilla_weather" {
                    // Llamar a la API de AEMET
                    match fetch_aemet_data(&api_key).await {
                        Ok(weather_data) => {
                            let response = json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "content": [
                                        {
                                            "type": "text",
                                            "text": format!("Datos de Sevilla obtenidos: {}", weather_data)
                                        }
                                    ]
                                }
                            });
                            println!("{}", response.to_string());
                        }
                        Err(e) => {
                            send_error(id, -32000, &format!("Error llamando a AEMET: {}", e));
                        }
                    }
                } else {
                    send_error(id, -32601, "Herramienta no encontrada");
                }
            }
            _ => {
                // Responder a otros métodos no implementados de forma genérica
                if !id.is_null() {
                    send_error(id, -32601, "Método no soportado");
                }
            }
        }

        line.clear();
    }

    Ok(())
}

// Función para interactuar con el endpoint de AEMET
async fn fetch_aemet_data(api_key: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    // Endpoint para la predicción específica de Sevilla (ejemplo: municipios de Sevilla capital es '41091')
    let url = "https://opendata.aemet.es/opendata/api/prediccion/especifica/municipio/diaria/41091";

    let res = client.get(url)
        .header("api_key", api_key)
        .send()
        .await?
        .json::<AemetResponse>()
        .await?;

    // AEMET devuelve un JSON con una URL ('datos') donde están los datos reales
    let data_res = client.get(&res.datos).send().await?.text().await?;
    
    Ok(data_res)
}

fn send_error(id: &serde_json::Value, code: i32, message: &str) {
    let error_response = json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    });
    println!("{}", error_response.to_string());
}
