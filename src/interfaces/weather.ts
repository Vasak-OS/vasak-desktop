export type WeatherPeriod = {
  description: string;
  image: string;
};

export type WeatherInfo = {
  day: WeatherPeriod;
  night: WeatherPeriod;
};

export type CodeDataType = {
  [key: string]: WeatherInfo;
};
