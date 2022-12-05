%% Pulse Bandwidth Calculations (THIS IS INCORRECT!!)
sample_period = 2e-6;
duration = 2664e-6;
n = floor(duration/sample_period);
pulse_waveform = sinc(linspace(-pi,pi,n));

freq_bin = 1/(sample_period*n);

f = abs(fft(pulse_waveform));
f = f./max(f) - 0.5;

f = f(1:numel(f)/2);

for i = 1:numel(f)-1
    if f(i) >= 0 && f(i+1) < 0
        break
    end
    if i==numel(f)-1
        error("zero-crossing not found");
    end
end
index = i+1; % measure based on below 50%


plot(f);hold on;
scatter(index,f(index))

bandwidth = (index-1)*freq_bin;

%%

Fs = 1000;            % Sampling frequency                    
T = 1/Fs;             % Sampling period       
L = 1500;             % Length of signal
t = (0:L-1)*T;        % Time vector

S = 0.7*sin(2*pi*50*t) + sin(2*pi*120*t);

X = S + 2*randn(size(t));

Y = fft(X);
%Compute the two-sided spectrum P2. Then compute the single-sided spectrum P1 based on P2 and the even-valued signal length L.

P2 = abs(Y/L);
P1 = P2(1:L/2+1);
P1(2:end-1) = 2*P1(2:end-1);
%Define the frequency domain f and plot the single-sided amplitude spectrum P1. The amplitudes are not exactly at 0.7 and 1, as expected, because of the added noise. On average, longer signals produce better frequency approximations.

f = Fs*(0:(L/2))/L;
plot(f,P1) 
title("Single-Sided Amplitude Spectrum of X(t)")
xlabel("f (Hz)")
ylabel("|P1(f)|")