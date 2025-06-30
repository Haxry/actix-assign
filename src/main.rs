use actix_web::{post, web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
};
use spl_token::instruction::{initialize_mint,mint_to};
use base64::{engine::general_purpose, Engine as _};
use base58::ToBase58;
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


#[post("/keypair")]
async fn generate_keypair() -> impl Responder {
    match Keypair::new() {
        keypair => {
            let pubkey = keypair.pubkey().to_string();
            let secret = keypair.to_bytes().to_vec().to_base58();

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
        &[], // No multisig
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


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!(" Server running on http://127.0.0.1:8080");

    HttpServer::new(|| {
        App::new()
            .service(generate_keypair)
            .service(create_token)
            .service(mint_token)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
