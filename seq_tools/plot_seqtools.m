function plot_seqtools(file)
rf_multplier = 3000;
s = fileread(file);
data = jsondecode(s);
% sort by data type
try
    f = gcf;
    cla
catch
   f = figure; 
end

grad_channel_colors = ["green","blue","red"];
acq_color = "black";
rf_color = "black";

%f.Position = [1300 258 560 420];
hold on
for i = 1:numel(data)
    if isfield(data(i).wave_data,'Rf')
        offset = data(i).waveform_start;
        t = data(i).wave_data.Rf(1).x + offset;
        amp = data(i).wave_data.Rf(1).y;
        plot(1000*t,rf_multplier*amp,'LineWidth',2,'Color',rf_color)
    end
    if isfield(data(i).wave_data,'Grad')
        offset = data(i).waveform_start;
        for j = 1:3
            wd = data(i).wave_data.Grad;
            if isstruct(wd)
                channel = wd(j);
            else
                channel = wd{j};
            end
            if ~isempty(channel)
                t = channel.x + offset;
                amp = channel.y;
                plot(1000*t,amp,'LineWidth',2,'Color',grad_channel_colors(j))
            end
        end
    end
    if isfield(data(i).wave_data,'Acq')
        offset = data(i).waveform_start;
        t = data(i).wave_data.Acq(1).x + offset;
        amp = data(i).wave_data.Acq(1).y;
        plot(1000*t,0.5*amp,'.','LineWidth',2,'Color',acq_color)
    end
end
hold off
pause(1);
