use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer,Signature},
    system_program,
    system_instruction
};
use spl_token::instruction::{initialize_mint,mint_to,transfer as spl_transfer};
use base64::{engine::general_purpose, Engine as _};
use base58::{ToBase58, FromBase58};
use std::str::FromStr;

#[derive(Serialize)]
struct SuccessResponse<T> {
    success: bool,
    data: T,
}

#[derive(Serialize)]
struct ErrorResponse {
    success: bool,
    error: String,
}

#[derive(Serialize)]
struct KeypairResponse {
    pubkey: String,
    secret: String,
}

#[derive(Deserialize)]
struct CreateTokenRequest {
    mintAuthority: String,
    mint: String,
    decimals: u8,
}

#[derive(Serialize)]
struct AccountMetaResponse {
    pubkey: String,
    is_signer: bool,
    is_writable: bool,
}

#[derive(Serialize)]
struct CreateTokenResponse {
    program_id: String,
    accounts: Vec<AccountMetaResponse>,
    instruction_data: String,
}

#[derive(Deserialize)]
struct MintTokenRequest {
    mint: String,
    destination: String,
    authority: String,
    amount: u64,
}



#[derive(Serialize)]
struct InstructionResponse {
    program_id: String,
    accounts: Vec<AccountMetaResponse>,
    instruction_data: String,
}


#[derive(Deserialize)]
struct SignMessageRequest {
    message: String,
    secret: String,
}

#[derive(Serialize)]
struct SignMessageResponse {
    signature: String,
    public_key: String,
    message: String,
}

#[derive(Deserialize)]
struct VerifyMessageRequest {
    message: String,
    signature: String,
    pubkey: String,
}

#[derive(Serialize)]
struct VerifyMessageResponse {
    valid: bool,
    message: String,
    pubkey: String,
}

#[derive(Deserialize)]
struct SendSolRequest {
    from: String,
    to: String,
    lamports: u64,
}

#[derive(Deserialize)]
struct SendTokenRequest {
    destination: String,
    mint: String,
    owner: String,
    amount: u64,
}

#[derive(Serialize)]
struct TokenAccountMeta {
    pubkey: String,
    is_signer: bool,
}


#[post("/keypair")]
async fn generate_keypair() -> impl Responder {
    match Keypair::new() {
        keypair => {
            let pubkey = keypair.pubkey().to_string();
            let secret = keypair.to_bytes().to_vec().to_base58();
            let bytes= keypair.to_bytes();
            print!("Generated Keypair: pubkey: {}, secret: {:?}", pubkey, bytes);

            HttpResponse::Ok().json(SuccessResponse {
                success: true,
                data: KeypairResponse { pubkey, secret },
            })
        }
    }
}

#[post("/token/create")]
async fn create_token(req: web::Json<CreateTokenRequest>) -> impl Responder {
    let mintAuthority = match Pubkey::from_str(&req.mintAuthority) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid mint authority pubkey".to_string(),
            });
        }
    };

    let mint = match Pubkey::from_str(&req.mint) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid mint pubkey".to_string(),
            });
        }
    };

    // Use system_program::id() as rent_sysvar is deprecated in Solana v2.2+
    let rent_sysvar = solana_sdk::sysvar::rent::id();

    let instr_result = initialize_mint(
        &spl_token::id(),
        &mint,
        &mintAuthority,
        None,
        req.decimals,
    );

    let instr = match instr_result {
        Ok(i) => i,
        Err(e) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: format!("Failed to build instruction: {e}"),
            });
        }
    };

    let accounts = instr
        .accounts
        .iter()
        .map(|meta| AccountMetaResponse {
            pubkey: meta.pubkey.to_string(),
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        })
        .collect();

    let instruction_data = general_purpose::STANDARD.encode(instr.data);

    let response = CreateTokenResponse {
        program_id: instr.program_id.to_string(),
        accounts,
        instruction_data,
    };

    HttpResponse::Ok().json(SuccessResponse {
        success: true,
        data: response,
    })
}

#[post("/token/mint")]
async fn mint_token(req: web::Json<MintTokenRequest>) -> impl Responder {
    let mint = match Pubkey::from_str(&req.mint) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid mint pubkey".to_string(),
            });
        }
    };

    let dest = match Pubkey::from_str(&req.destination) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid destination pubkey".to_string(),
            });
        }
    };

    let authority = match Pubkey::from_str(&req.authority) {
        Ok(pk) => pk,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: "Invalid authority pubkey".to_string(),
            });
        }
    };

    let instr = match mint_to(
        &spl_token::id(),
        &mint,
        &dest,
        &authority,
        &[], 
        req.amount,
    ) {
        Ok(i) => i,
        Err(e) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: format!("Failed to create instruction: {}", e),
            });
        }
    };

    let accounts = instr
        .accounts
        .iter()
        .map(|meta| AccountMetaResponse {
            pubkey: meta.pubkey.to_string(),
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        })
        .collect::<Vec<_>>();

    let instruction_data = general_purpose::STANDARD.encode(instr.data);

    let response = InstructionResponse {
        program_id: instr.program_id.to_string(),
        accounts,
        instruction_data,
    };

    HttpResponse::Ok().json(SuccessResponse {
        success: true,
        data: response,
    })
}

// fn base58_to_base64(base58_str: &str) -> Result<String, Box<dyn std::error::Error>> {
//     let raw_bytes = bs58::decode(base58_str).into_vec()?;
//     let base64_str = general_purpose::STANDARD.encode(&raw_bytes);
//     Ok(base64_str)
// }

// fn base64_to_base58(base64_str: &str) -> Result<String, Box<dyn std::error::Error>> {
//     let raw_bytes = general_purpose::STANDARD.decode(base64_str)?;
//     let base58_str = bs58::encode(&raw_bytes).into_string();
//     Ok(base58_str)
// }

#[post("/message/sign")]
async fn sign_message(req: web::Json<SignMessageRequest>) -> impl Responder {
    if req.message.is_empty() || req.secret.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Missing required fields"
        }));
    }

    let secret_bytes = match req.secret.from_base58() {
        Ok(b) => b,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "Invalid base58 secret key"
            }));
        }
    };

    // let base64_secret = match base58_to_base64(&req.secret) {
    //     Ok(b64) => b64,
    //     Err(_) => {
    //         return HttpResponse::BadRequest().json(serde_json::json!({
    //             "success": false,
    //             "error": "Failed to convert secret key to base64"
    //         }));
    //     }
    // };

    let keypair = match Keypair::from_bytes(&secret_bytes) {
        Ok(kp) => kp,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "Missing required fields"
            }));
        }
    };

    let signature = keypair.sign_message(req.message.as_bytes());

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": {
            "signature": general_purpose::STANDARD.encode(signature.as_ref()),
            "public_key": keypair.pubkey().to_string(),
            "message": req.message
        }
    }))
}

#[post("/message/verify")]
async fn verify_message(req: web::Json<VerifyMessageRequest>) -> impl Responder {
    let pubkey = match Pubkey::from_str(&req.pubkey) {
        Ok(p) => p,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "Invalid public key"
            }));
        }
    };

    let signature_bytes = match general_purpose::STANDARD.decode(&req.signature) {
        Ok(bytes) => bytes,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "Invalid base64 signature"
            }));
        }
    };

    let signature = match Signature::try_from(signature_bytes.as_slice()) {
        Ok(sig) => sig,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "error": "Invalid signature format"
            }));
        }
    };

    let valid = signature.verify(pubkey.as_ref(), req.message.as_bytes());

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": {
            "valid": valid,
            "message": req.message,
            "pubkey": pubkey.to_string()
        }
    }))
}

#[post("/send/sol")]
async fn send_sol(req: web::Json<SendSolRequest>) -> impl Responder {
    let from = match Pubkey::from_str(&req.from) {
        Ok(pk) => pk,
        Err(_) => return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Invalid sender address"
        })),
    };

    let to = match Pubkey::from_str(&req.to) {
        Ok(pk) => pk,
        Err(_) => return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Invalid recipient address"
        })),
    };

    let instr = system_instruction::transfer(&from, &to, req.lamports);

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": {
            "program_id": instr.program_id.to_string(),
            "accounts": [
                instr.accounts[0].pubkey.to_string(),
                instr.accounts[1].pubkey.to_string()
            ],
            "instruction_data": general_purpose::STANDARD.encode(instr.data)
        }
    }))
}

#[post("/send/token")]
async fn send_token(req: web::Json<SendTokenRequest>) -> impl Responder {
    let destination = match Pubkey::from_str(&req.destination) {
        Ok(pk) => pk,
        Err(_) => return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Invalid destination address"
        })),
    };

    let mint = match Pubkey::from_str(&req.mint) {
        Ok(pk) => pk,
        Err(_) => return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Invalid mint address"
        })),
    };

    let owner = match Pubkey::from_str(&req.owner) {
        Ok(pk) => pk,
        Err(_) => return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": "Invalid owner address"
        })),
    };

    let instr = match spl_transfer(
        &spl_token::id(),
        &owner,         
        &destination,  
        &owner,         
        &[],            
        req.amount,
    ) {
        Ok(instr) => instr,
        Err(e) => return HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "error": format!("Failed to build transfer: {}", e)
        })),
    };

    let accounts = instr
        .accounts
        .iter()
        .map(|meta| TokenAccountMeta {
            pubkey: meta.pubkey.to_string(),
            is_signer: meta.is_signer,
        })
        .collect::<Vec<_>>();

    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": {
            "program_id": instr.program_id.to_string(),
            "accounts": accounts,
            "instruction_data": general_purpose::STANDARD.encode(instr.data)
        }
    }))
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!(" Server running on http://127.0.0.1:8080");

    HttpServer::new(|| {
        App::new()
            .service(generate_keypair)
            .service(create_token)
            .service(mint_token)
            .service(sign_message)
            .service(verify_message)
            .service(send_sol)
            .service(send_token)
    })
    .bind(("0.0.0.0:8080"))?
    .run()
    .await
}
