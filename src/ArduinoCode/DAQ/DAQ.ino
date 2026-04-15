#include <SparkFun_UHF_RFID_Reader.h>

#include <SD.h>
#include <SPI.h>
#include <Math.h>
#include <String.h>

//rfid module 
#include <SoftwareSerial.h> 
#define rfidSerial softSerial
#define rfidBaud 38400
//#define rfidBaud 115200
#define moduleType ThingMagic_M7E_HECTO

#define SD_CS 10  //SD card port
#define TRIG 5    //Trigger pin for reader

RFID rfidModule;
struct TAG_t{
  int rssi = rfidModule.getTagRSSI();
  long freq = rfidModule.getTagFreq();
  long timeStamp = rfidModule.getTagTimestamp();
  byte tagEPC = rfidModule.getTagEPCBytes();
}TAG;

SoftwareSerial softSerial(2,3); //RX, TX

//SD values 
char* filename;
File dataFile;
bool SDconn = 0;

void setup(){

  Serial.begin(115200);
  SDsetup();
  pinMode(TRIG, INPUT);
  if (setupRfidModule(rfidBaud) == false)
  {
    Serial.println(F("Module failed to respond. Please check wiring."));
    while (1); //Freeze!
  }

  rfidModule.setRegion(REGION_NORTHAMERICA); //Set to North America

  rfidModule.setReadPower(2700); //5.00 dBm. Higher values may caues USB port to brown out
  //Max Read TX Power is 27.00 dBm and may cause temperature-limit throttling

  Serial.println(F("Modual connected! Constant scanning started."));
  

  rfidModule.startReading(); //Begin scanning for tags


}

void loop(){
  //trigger is here for later 
  //if (digitalRead(TRIG) == HIGH){
    if (rfidModule.check() == true) //Check to see if any new data has come in from module
    {
      byte responseType = rfidModule.parseResponse(); //Break response into tag ID, RSSI, frequency, and timestamp

      if (responseType == RESPONSE_IS_KEEPALIVE)
      {
        Serial.println(F("Scanning"));
      }
      else if (responseType == RESPONSE_IS_TAGFOUND)
      {
        //If we have a full record we can pull out the fun bits
        int rssi = rfidModule.getTagRSSI(); //Get the RSSI for this tag read

        long freq = rfidModule.getTagFreq(); //Get the frequency this tag was detected at

        long timeStamp = rfidModule.getTagTimestamp(); //Get the time this was read, (ms) since last keep-alive message

        byte tagEPCBytes = rfidModule.getTagEPCBytes(); //Get the number of bytes of EPC from response

        SDwrite(rssi, freq, timeStamp, tagEPCBytes);
        Serial.print(F(" rssi["));
        Serial.print(rssi);
        Serial.print(F("]"));

        Serial.print(F(" freq["));
        Serial.print(freq);
        Serial.print(F("]"));

        Serial.print(F(" time["));
        Serial.print(timeStamp);
        Serial.print(F("]"));
        //Print EPC bytes, this is a subsection of bytes from the response/msg array
        Serial.print(F(" epc["));
        for (byte x = 0 ; x < tagEPCBytes ; x++)
        {
          if (rfidModule.msg[31 + x] < 0x10){
            Serial.print(F("0"));
          }  //Pretty print
          Serial.print(rfidModule.msg[31 + x], HEX);
          Serial.print(F(" "));
        }
        Serial.print(F("]"));
        Serial.println();
        delay(500);
      }
      else if (responseType == ERROR_CORRUPT_RESPONSE)
      {
        Serial.println("Bad CRC");
      }
      else if (responseType == RESPONSE_IS_HIGHRETURNLOSS)
      {
        Serial.println("High return loss, check antenna!");
      }
      else
      {
        //Unknown response
        Serial.println("Unknown error");
      }
    }
  //}
}

// sets up the SD card for writing with a log txt file and directory
void SDsetup(){
  if (SD.begin(SD_CS) == 1){
    Serial.println("SD card connected...");
    if (!SD.exists("EchoNet")){
      SD.mkdir("EchoNet");
    }
    filename = "EchoNet/log_1.txt";
    uint8_t i = 0;
    if(SD.exists(filename)){
      while(SD.exists(filename)){
        i++;
        sprintf(filename, "EchoNet/log_%02d.txt", i);
        
      }
    } else {
      filename = "EchoNet/log_1.txt";
    }
    dataFile = SD.open(filename, FILE_WRITE);
    dataFile.close();
    Serial.println("File Created");
    SDconn = 1;
  } else {
    Serial.println("SD card failed to connect...");
  }
  dataFile = SD.open(filename, FILE_WRITE);
}

void SDwrite(int rssi, long freq, long time, byte epc){
  if (SDconn == 1){
    Serial.print("SDwrite");
    dataFile.print("RSSI: %d");
    dataFile.print(rssi);
    dataFile.print(" Frequency: ");
    dataFile.print(freq);
    dataFile.print(" Timestamp: ");
    dataFile.print(time);
    dataFile.print(" EPC[ ");
    for (byte x = 0 ; x < epc ; x++){
      if (rfidModule.msg[31 + x] < 0x10){
        dataFile.print(F("0"));
      }  //Pretty print
      dataFile.print(rfidModule.msg[31 + x], HEX);
      dataFile.print(F(" "));
    }
    dataFile.print(F("]"));
    dataFile.println();
    dataFile.flush();
  } else {
    Serial.println("Erm... no SD card...");
  }
}

boolean setupRfidModule(long baudRate)
{
  rfidModule.begin(rfidSerial, moduleType); //Tell the library to communicate over serial port
  //Test to see if we are already connected to a module
  //This would be the case if the Arduino has been reprogrammed and the module has stayed powered
  rfidSerial.begin(baudRate); //For this test, assume module is already at our desired baud rate
  delay(100); //Wait for port to open
  //About 200ms from power on the module will send its firmware version at 115200. We need to ignore this.
  while (rfidSerial.available())
    rfidSerial.read();

  rfidModule.getVersion();

  if (rfidModule.msg[0] == ERROR_WRONG_OPCODE_RESPONSE)
  {
    //This happens if the baud rate is correct but the module is doing a ccontinuous read
    rfidModule.stopReading();
    Serial.println(F("Module continuously reading. Asking it to stop..."));
    delay(1500);
  }
  else
  {
    //The module did not respond so assume it's just been powered on and communicating at 115200bps
    rfidSerial.begin(115200); //Start serial at 115200
    rfidModule.setBaud(baudRate); //Tell the module to go to the chosen baud rate. Ignore the response msg
    rfidSerial.begin(baudRate); //Start the serial port, this time at user's chosen baud rate
    delay(250);
  }

  //Test the connection
  rfidModule.getVersion();
  if (rfidModule.msg[0] != ALL_GOOD)
    return false; //Something is not right

  //The module has these settings no matter what
  rfidModule.setTagProtocol(); //Set protocol to GEN2
  rfidModule.setAntennaPort(); //Set TX/RX antenna ports to 1
  return true; //We are ready to rock
}