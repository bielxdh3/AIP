package com.aip.companion
import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.spec.GCMParameterSpec
class SecureStore(c:Context){private val p=c.getSharedPreferences("companion-meta",0);private val alias="aip-pairing-v1";private fun key()=KeyStore.getInstance("AndroidKeyStore").let{it.load(null);(it.getKey(alias,null)?:KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES,"AndroidKeyStore").apply{init(KeyGenParameterSpec.Builder(alias,KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT).setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build())}.generateKey())};fun save(secret:ByteArray){val x=Cipher.getInstance("AES/GCM/NoPadding");x.init(Cipher.ENCRYPT_MODE,key());p.edit().putString("encrypted_secret",Base64.encodeToString(x.iv+x.doFinal(secret),2)).apply()};fun load():ByteArray?=p.getString("encrypted_secret",null)?.let{val b=Base64.decode(it,2);Cipher.getInstance("AES/GCM/NoPadding").apply{init(Cipher.DECRYPT_MODE,key(),GCMParameterSpec(128,b.copyOfRange(0,12)))}.doFinal(b.copyOfRange(12,b.size))};fun hasSecret()=p.contains("encrypted_secret")}
