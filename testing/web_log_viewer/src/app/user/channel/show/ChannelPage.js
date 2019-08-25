import React from 'react';
import PropTypes from 'prop-types';
import { withStyles } from '@material-ui/core/styles';
import Typography from '@material-ui/core/Typography';
import SimpleLineChart from './ChannelChart';
import Card from '@material-ui/core/Card';
import CardActionArea from '@material-ui/core/CardActionArea';
import CardActions from '@material-ui/core/CardActions';
import CardContent from '@material-ui/core/CardContent';
import CardMedia from '@material-ui/core/CardMedia';
import errorImage from '../../../../images/error.png'
import CircularProgress from '@material-ui/core/CircularProgress';
import SimpleTable from './ChannelDataTable';
import { styles } from '../../shared/styles';
import { getChannelData } from './actions';
import { connect } from 'react-redux';
import { withRouter } from 'react-router-dom';
import { secondaryMenuContent } from './secondaryMenuContent'
import { Button } from '@material-ui/core';

class Channel extends React.Component {

  state = {
    isLoading: false,
    error: '',
    data: '',
  };


  componentDidMount() {
    this.props.titleHandler("Node Logs")
    this.props.secondaryMenu(secondaryMenuContent)
    this.props.getChannelData(this.props.history.location.state.host);
  }

  reloadData() {
    this.props.getChannelData(this.props.history.location.state.host);
    this.setState({ isLoading: true, error: '', data: '', });
  }

  render() {
    const { classes } = this.props;
    if (this.props.error)
      return (
      
      <Card className={classes.errorCard}>
        <CardActionArea>
          <CardContent>
            <img src={errorImage} className={classes.errorImage} alt='error icon' />
            <Typography gutterBottom variant="h5" component="h2">
              Error while loading channels</Typography>
            <Typography component="p">
              {this.props.error}</Typography>
          </CardContent>
        </CardActionArea>
      </Card>)


    if (this.props.chartData) {
      var data = this.convertDataToChartData();
      return (
        <React.Fragment>
          <Typography component="div" className={classes.chartContainer}>
            <SimpleLineChart chartData={data} />
          </Typography>
          <div className={classes.buttons}>
            <Button
              type="submit"
              variant="contained"
              color="primary"
              className={classes.button}
              onClick={() => this.reloadData()}
            >Refresh Data
                        </Button>
          </div>
          <React.Fragment>
            <div style={{ maxWidth: "100%" }}>
              <SimpleTable chartData={this.props.chartData} />
            </div>
          </React.Fragment>
        </React.Fragment>
      );
    }
    return this.renderProgressComponent();
  }

  convertDataToChartData() {

    let data = this.props.chartData.sort((a, b) => (a.scheduled_at_date > b.scheduled_at_date) ? 1 : ((b.scheduled_at_date > a.scheduled_at_date) ? -1 : 0));
    return data.map(
      measurement => (
        {
          name: new Date(measurement.created_at_time).toLocaleString("en-US"),
          block_id: measurement.scheduled_at_date
        }));
  }

  renderProgressComponent() {
    const { classes } = this.props;

    return (
      <React.Fragment>
        <CircularProgress className={classes.progress} size={100} />
      </React.Fragment>)
  }
}

Channel.propTypes = {
  classes: PropTypes.object.isRequired,
};

const mapStateToProps = state => {
  return {
    chartData: state.channel.data.data,
    error: state.channel.error,
    isLoading: state.channel.isLoading
  }
}

export default connect(mapStateToProps, { getChannelData })(withStyles(styles)(withRouter(Channel)))